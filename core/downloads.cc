#include "browser_core/downloads.h"

#include <algorithm>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "base/functional/bind.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/supports_user_data.h"
#include "base/task/cancelable_task_tracker.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "chrome/browser/browser_process.h"
#include "chrome/browser/download/download_crx_util.h"
#include "chrome/browser/download/download_item_model.h"
#include "chrome/browser/icon_loader.h"
#include "chrome/browser/icon_manager.h"
#include "chrome/browser/profiles/profile.h"
#include "components/download/content/public/all_download_item_notifier.h"
#include "components/download/public/common/download_item.h"
#include "content/public/browser/download_manager.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/image/image.h"
#include "ui/gfx/image/image_skia.h"
#include "ui/gfx/image/image_skia_rep.h"

namespace browser_core {

namespace {

// Ours: Arc's limit isn't known. Chrome's history keeps downloads without end.
constexpr size_t kMaxListed = 500;

// The id the app knows a download by, kept on it.
struct DownloadId : base::SupportsUserData::Data {
  static constexpr char kKey[] = "browser_core::DownloadId";
  bc_id id = NewId();
};

bc_id IdOf(download::DownloadItem* item) {
  auto* data = static_cast<DownloadId*>(item->GetUserData(DownloadId::kKey));
  if (!data) {
    auto id = std::make_unique<DownloadId>();
    data = id.get();
    item->SetUserData(DownloadId::kKey, std::move(id));
  }
  return data->id;
}

int StateOf(const download::DownloadItem& item) {
  switch (item.GetState()) {
    case download::DownloadItem::IN_PROGRESS:
      return item.IsPaused() ? BC_DOWNLOAD_PAUSED : BC_DOWNLOAD_RUNNING;
    case download::DownloadItem::COMPLETE:
      return BC_DOWNLOAD_DONE;
    case download::DownloadItem::CANCELLED:
      return BC_DOWNLOAD_CANCELLED;
    default:
      return BC_DOWNLOAD_FAILED;
  }
}

class Downloads : public download::AllDownloadItemNotifier::Observer {
 public:
  explicit Downloads(Profile* profile) {
    // Made here, as it tells of a manager that has read the history at once.
    notifier_ = std::make_unique<download::AllDownloadItemNotifier>(
        profile->GetDownloadManager(), this);
    Changed();
  }

  // The ones Chrome's downloads page lists, newest first.
  std::vector<download::DownloadItem*> Listed() {
    content::DownloadManager* manager = notifier_->GetManager();
    if (!manager) {
      return {};
    }
    Profile* profile = Profile::FromBrowserContext(manager->GetBrowserContext());
    content::DownloadManager::DownloadVector all;
    manager->GetAllDownloads(&all);
    std::vector<download::DownloadItem*> listed;
    for (download::DownloadItem* item : all) {
      if (!download_crx_util::IsTrustedExtensionDownload(profile, *item) &&
          !item->IsTemporary() && !item->IsTransient() &&
          !item->GetFileNameToReportUser().empty() && !item->GetTargetFilePath().empty() &&
          !item->GetURL().is_empty() && DownloadItemModel(item).ShouldShowInUi()) {
        listed.push_back(item);
      }
    }
    std::ranges::sort(listed, std::ranges::greater(), &download::DownloadItem::GetStartTime);
    if (listed.size() > kMaxListed) {
      listed.resize(kMaxListed);
    }
    return listed;
  }

  download::DownloadItem* Find(bc_id id) {
    for (download::DownloadItem* item : Listed()) {
      if (IdOf(item) == id) {
        return item;
      }
    }
    return nullptr;
  }

 private:
  // download::AllDownloadItemNotifier::Observer:
  void OnManagerInitialized(content::DownloadManager* manager) override { Changed(); }
  void OnDownloadCreated(content::DownloadManager* manager,
                         download::DownloadItem* item) override {
    Changed();
  }
  void OnDownloadUpdated(content::DownloadManager* manager,
                         download::DownloadItem* item) override {
    Changed();
  }
  void OnDownloadRemoved(content::DownloadManager* manager,
                         download::DownloadItem* item) override {
    Changed();
  }

  // Tells the app once, after the call that changed them has returned. A running download changes
  // about twice a second.
  void Changed() {
    if (report_posted_) {
      return;
    }
    report_posted_ = true;
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Downloads::Report, weak_factory_.GetWeakPtr()));
  }

  void Report() {
    report_posted_ = false;
    if (!Host().downloads_changed) {
      return;
    }
    const std::vector<download::DownloadItem*> items = Listed();
    // Each download's name and URL, kept until the call returns.
    std::vector<std::string> text;
    text.reserve(items.size() * 2);
    std::vector<bc_download> downloads;
    downloads.reserve(items.size());
    for (download::DownloadItem* item : items) {
      const std::string& name = text.emplace_back(item->GetFileNameToReportUser().AsUTF8Unsafe());
      const std::string& url = text.emplace_back(item->GetURL().spec());
      downloads.push_back({.id = IdOf(item),
                           .name = name.c_str(),
                           .url = url.c_str(),
                           .received = item->GetReceivedBytes(),
                           .total = item->GetTotalBytes(),
                           .state = StateOf(*item),
                           .open_when_done = item->GetOpenWhenComplete(),
                           .held = item->IsDangerous() || item->IsInsecure(),
                           .started = item->GetStartTime().InMillisecondsFSinceUnixEpoch()});
    }
    Host().downloads_changed(Host().context, downloads.data(), downloads.size());
  }

  std::unique_ptr<download::AllDownloadItemNotifier> notifier_;
  bool report_posted_ = false;
  base::WeakPtrFactory<Downloads> weak_factory_{this};
};

std::unique_ptr<Downloads>& TheDownloads() {
  static base::NoDestructor<std::unique_ptr<Downloads>> downloads;
  return *downloads;
}

download::DownloadItem* Find(bc_id id) {
  return TheDownloads() ? TheDownloads()->Find(id) : nullptr;
}

void ReportIcon(bc_id id, gfx::Image icon) {
  if (!Host().download_icon) {
    return;
  }
  std::optional<std::vector<uint8_t>> png;
  if (!icon.IsEmpty()) {
    png = gfx::PNGCodec::EncodeBGRASkBitmap(icon.AsImageSkia().GetRepresentation(2).GetBitmap(),
                                            /*discard_transparency=*/false);
  }
  Host().download_icon(Host().context, id, png ? png->data() : nullptr, png ? png->size() : 0);
}

}  // namespace

void ObserveDownloads(Profile* profile) {
  TheDownloads() = std::make_unique<Downloads>(profile);
}

void ActOnDownload(bc_id id, int action) {
  download::DownloadItem* item = Find(id);
  if (!item) {
    return;
  }
  switch (action) {
    case BC_DOWNLOAD_OPEN:
      // A download still running opens once it's done, or no longer.
      item->OpenDownload();
      break;
    case BC_DOWNLOAD_SHOW:
      item->ShowDownloadInShell();
      break;
    case BC_DOWNLOAD_PAUSE:
      item->Pause();
      break;
    case BC_DOWNLOAD_RESUME:
      item->Resume(/*user_resume=*/true);
      break;
    case BC_DOWNLOAD_CANCEL:
      item->Cancel(/*user_cancel=*/true);
      break;
    case BC_DOWNLOAD_REMOVE:
      item->Remove();
      break;
    case BC_DOWNLOAD_KEEP:
      if (item->IsInsecure()) {
        item->ValidateInsecureDownload();
      } else if (item->IsDangerous()) {
        item->ValidateDangerousDownload();
      }
      break;
  }
}

void ClearDownloads() {
  if (!TheDownloads()) {
    return;
  }
  for (download::DownloadItem* item : TheDownloads()->Listed()) {
    if (item->GetState() != download::DownloadItem::IN_PROGRESS) {
      item->Remove();
    }
  }
}

void FindDownloadIcon(bc_id id) {
  download::DownloadItem* item = Find(id);
  IconManager* icons = g_browser_process->icon_manager();
  if (!item || !icons) {
    return;
  }
  static base::NoDestructor<base::CancelableTaskTracker> tracker;
  // 16 points at 2×, as favicons are.
  icons->LoadIcon(item->GetTargetFilePath(), IconLoader::SMALL, 2,
                  base::BindOnce(&ReportIcon, id), tracker.get());
}

}  // namespace browser_core
