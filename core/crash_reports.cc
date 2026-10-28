#include "browser_core/crash_reports.h"

#include <algorithm>
#include <cstdio>
#include <iterator>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

#include "base/command_line.h"
#include "base/debug/dump_without_crashing.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/immediate_crash.h"
#include "base/location.h"
#include "base/memory/scoped_refptr.h"
#include "base/no_destructor.h"
#include "base/path_service.h"
#include "base/task/sequenced_task_runner.h"
#include "base/task/single_thread_task_runner.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "browser_core/session.h"
#include "build/build_config.h"
#include "chrome/browser/after_startup_task_utils.h"
#include "chrome/browser/browser_process.h"
#include "chrome/common/chrome_paths.h"
#include "chrome/common/chrome_switches.h"
#include "components/crash/core/app/crashpad.h"
#include "net/base/mime_util.h"
#include "net/http/http_response_headers.h"
#include "net/traffic_annotation/network_traffic_annotation.h"
#include "services/network/public/cpp/resource_request.h"
#include "services/network/public/cpp/shared_url_loader_factory.h"
#include "services/network/public/cpp/simple_url_loader.h"
#include "services/network/public/mojom/fetch_api.mojom-shared.h"
#include "third_party/crashpad/crashpad/client/crash_report_database.h"
#include "third_party/crashpad/crashpad/client/crashpad_info.h"
#include "url/gurl.h"

#if BUILDFLAG(IS_MAC)
#include <mach/mach.h>

#include "base/apple/scoped_mach_port.h"
#include "third_party/crashpad/crashpad/client/crashpad_client.h"
#endif

namespace browser_core::crash_reports {

namespace {

using crashpad::CrashReportDatabase;

// Not Chrome's --crash-test, which crashes the browser before its first window.
constexpr char kCrashTest[] = "crash-after-start";
constexpr char kCollector[] = "crash-upload-url";

constexpr net::NetworkTrafficAnnotationTag kCrashReport =
    net::DefineNetworkTrafficAnnotation("browser_core_crash_report", R"(
      semantics {
        sender: "Crash reports"
        description:
          "Sends Crashpad's minidump of a crash to the crash collector the build names, so its "
          "developers can fix it."
        trigger:
          "The user has Settings › Privacy › Send Crash Reports on, and a report hasn't been sent "
          "yet. Checked once the browser has started, and when the user turns it on."
        data:
          "The minidump alone: the crashed process's threads, their stacks and registers, its "
          "loaded modules and Crashpad's annotations, such as the browser's version."
        destination: OTHER
        destination_other: "The collector the build names, which anyone can host."
      }
      policy {
        cookies_allowed: NO
        setting: "Settings › Privacy › Send Crash Reports, off by default."
        policy_exception_justification: "Off unless the user turns it on."
      })");

const GURL& Collector() {
  static const base::NoDestructor<GURL> collector([] {
    GURL url(base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(kCollector));
    return url.is_valid() && url.SchemeIsHTTPOrHTTPS() ? url : GURL();
  }());
  return *collector;
}

// Where the reports are read and deleted: off the UI thread, one thing at a time, and never in the
// way of what the user is doing.
scoped_refptr<base::SequencedTaskRunner> Reports() {
  static base::NoDestructor<scoped_refptr<base::SequencedTaskRunner>> reports(
      base::ThreadPool::CreateSequencedTaskRunner(
          {base::MayBlock(), base::TaskPriority::BEST_EFFORT,
           base::TaskShutdownBehavior::SKIP_ON_SHUTDOWN}));
  return *reports;
}

// The Crashpad database the handler writes, as another reader of it, as Chrome's crashes page is.
std::unique_ptr<CrashReportDatabase> OpenReports() {
  std::optional<base::FilePath> path = crash_reporter::GetCrashpadDatabasePath();
  return path ? CrashReportDatabase::InitializeWithoutCreating(*path) : nullptr;
}

struct Unsent {
  crashpad::UUID uuid;
  std::string minidump;
};

// The oldest report not yet sent, read whole. One that can't be read is deleted.
std::optional<Unsent> ReadOldest() {
  std::unique_ptr<CrashReportDatabase> reports = OpenReports();
  std::vector<CrashReportDatabase::Report> pending;
  if (!reports || reports->GetPendingReports(&pending) != CrashReportDatabase::kNoError) {
    return std::nullopt;
  }
  std::ranges::sort(pending, {}, &CrashReportDatabase::Report::creation_time);
  for (const CrashReportDatabase::Report& report : pending) {
    std::string minidump;
    if (base::ReadFileToString(report.file_path, &minidump)) {
      return Unsent{report.uuid, std::move(minidump)};
    }
    reports->DeleteReport(report.uuid);
  }
  return std::nullopt;
}

void Delete(const crashpad::UUID& uuid) {
  if (std::unique_ptr<CrashReportDatabase> reports = OpenReports()) {
    reports->DeleteReport(uuid);
  }
}

// Every report the collector doesn't have: the handler's pending ones, and any Chromium recorded as
// not sent.
void DeleteUnsent() {
  std::unique_ptr<CrashReportDatabase> reports = OpenReports();
  if (!reports) {
    return;
  }
  std::vector<CrashReportDatabase::Report> unsent;
  std::vector<CrashReportDatabase::Report> completed;
  reports->GetPendingReports(&unsent);
  reports->GetCompletedReports(&completed);
  std::ranges::copy_if(completed, std::back_inserter(unsent),
                       [](const CrashReportDatabase::Report& report) { return !report.uploaded; });
  int deleted = 0;
  for (const CrashReportDatabase::Report& report : unsent) {
    deleted += reports->DeleteReport(report.uuid) == CrashReportDatabase::kNoError;
  }
  // check.sh waits for this line.
  fprintf(stderr, "browser: crash reports deleted %d\n", deleted);
}

// Whether a report is on its way. Reports go one at a time, so none goes twice.
bool g_busy = false;
// Counts the times the user turned sending off, so what was on its way then stops where it is.
int g_round = 0;

std::unique_ptr<network::SimpleURLLoader>& Loader() {
  static base::NoDestructor<std::unique_ptr<network::SimpleURLLoader>> loader;
  return *loader;
}

void SendNext();

void Sent(int round,
          const crashpad::UUID& uuid,
          scoped_refptr<net::HttpResponseHeaders> headers) {
  if (round != g_round) {
    return;
  }
  Loader().reset();
  g_busy = false;
  if (!headers || headers->response_code() / 100 != 2) {
    // check.sh waits for this line.
    fprintf(stderr, "browser: crash report not sent\n");
    return;
  }
  // check.sh waits for this line.
  fprintf(stderr, "browser: crash report sent\n");
  Reports()->PostTaskAndReply(FROM_HERE, base::BindOnce(&Delete, uuid), base::BindOnce(&SendNext));
}

// The minidump alone, as a Breakpad collector takes it, which GlitchTip's and Sentry's minidump
// endpoints are. Crashpad's handler would add the minidump's annotations as fields, which the
// minidump has anyway.
void Send(int round, std::optional<Unsent> report) {
  if (round != g_round) {
    return;
  }
  if (!report || !Sending() || !g_browser_process) {
    g_busy = false;
    return;
  }
  const std::string boundary = net::GenerateMimeMultipartBoundary();
  std::string body;
  net::AddMultipartValueForUploadWithFileName("upload_file_minidump",
                                              report->uuid.ToString() + ".dmp", report->minidump,
                                              boundary, "application/octet-stream", &body);
  net::AddMultipartFinalDelimiterForUpload(boundary, &body);
  auto request = std::make_unique<network::ResourceRequest>();
  request->url = Collector();
  request->method = "POST";
  request->credentials_mode = network::mojom::CredentialsMode::kOmit;
  Loader() = network::SimpleURLLoader::Create(std::move(request), kCrashReport);
  Loader()->AttachStringForUpload(std::move(body), "multipart/form-data; boundary=" + boundary);
  Loader()->DownloadHeadersOnly(g_browser_process->shared_url_loader_factory().get(),
                                base::BindOnce(&Sent, round, report->uuid));
}

void SendNext() {
  if (g_busy || !Sending()) {
    return;
  }
  g_busy = true;
  Reports()->PostTaskAndReplyWithResult(FROM_HERE, base::BindOnce(&ReadOldest),
                                        base::BindOnce(&Send, g_round));
}

// Whether Crashpad's handler takes crashes yet. On the Mac the browser doesn't wait for it to start
// (chromium-crashpad-mac-async-start.patch): it takes them once it has the receive right of the
// port the browser's crashes go to.
bool HandlerReady() {
#if BUILDFLAG(IS_MAC)
  base::apple::ScopedMachSendRight port =
      crash_reporter::GetCrashpadClient().GetHandlerMachPort();
  mach_port_type_t type = 0;
  return port.is_valid() && mach_port_type(mach_task_self(), port.get(), &type) == KERN_SUCCESS &&
         !(type & MACH_PORT_TYPE_RECEIVE);
#else
  return true;
#endif
}

void CrashOnceReady() {
  if (!HandlerReady()) {
    // Only with the test's switch, so no other launch wakes for this.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostDelayedTask(
        FROM_HERE, base::BindOnce(&CrashOnceReady), base::Milliseconds(50));
    return;
  }
  // check.sh waits for this line.
  fprintf(stderr, "browser: crash test\n");
  // A crash asked for is no news for macOS's crash reporter to show the user.
  crashpad::CrashpadInfo::GetCrashpadInfo()->set_system_crash_reporter_forwarding(
      crashpad::TriState::kDisabled);
  base::ImmediateCrash();
}

}  // namespace

void PlaceReports() {
  base::FilePath profiles =
      base::CommandLine::ForCurrentProcess()->GetSwitchValuePath(switches::kUserDataDir);
  if (!profiles.empty()) {
    base::PathService::Override(chrome::DIR_CRASH_DUMPS,
                                profiles.Append(FILE_PATH_LITERAL("Crashpad")));
  }
}

void LeaveOutPages() {
  crashpad::CrashpadInfo::GetCrashpadInfo()->set_gather_indirectly_referenced_memory(
      crashpad::TriState::kDisabled, 0);
  base::debug::SetDumpWithoutCrashingFunction(nullptr);
}

std::string CollectorHost() {
  return Collector().host();
}

bool Sending() {
  return UserSettings().send_crash_reports && Collector().is_valid();
}

void SetSending(bool send) {
  if (UserSettings().send_crash_reports == send) {
    return;
  }
  UserSettings().send_crash_reports = send;
  SessionChanged();
  if (send) {
    SendNext();
    return;
  }
  ++g_round;
  g_busy = false;
  Loader().reset();
  Reports()->PostTask(FROM_HERE, base::BindOnce(&DeleteUnsent));
}

void Started() {
  if (base::CommandLine::ForCurrentProcess()->HasSwitch(kCrashTest)) {
    // Posted, so the app has done with being told it started.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE,
                                                                base::BindOnce(&CrashOnceReady));
    return;
  }
  if (Sending()) {
    AfterStartupTaskUtils::PostTask(FROM_HERE, base::SequencedTaskRunner::GetCurrentDefault(),
                                    base::BindOnce(&SendNext));
  }
}

}  // namespace browser_core::crash_reports
