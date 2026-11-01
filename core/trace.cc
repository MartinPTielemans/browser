// Record a Trace (docs/ux-spec.md › Record a Trace): Chromium's trace of every process, which only
// the user starts and stops, and which goes only to the app, to save where the user picks.

#include <memory>
#include <string>

#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/trace_event/trace_config.h"
#include "browser_core/host.h"
#include "browser_core/public/browser_core.h"
#include "content/public/browser/tracing_controller.h"

bool bc_trace_start(void) {
  content::TracingController* tracing = content::TracingController::GetInstance();
  if (tracing->IsTracing()) {
    return false;
  }
  // Chrome's default categories, into a buffer that keeps the latest events once it's full, so a
  // recording left running holds its last minutes rather than growing.
  return tracing->StartTracing(
      base::trace_event::TraceConfig("", base::trace_event::RECORD_CONTINUOUSLY),
      base::DoNothing());
}

void bc_trace_stop(void) {
  content::TracingController* tracing = content::TracingController::GetInstance();
  if (!tracing->IsTracing()) {
    return;
  }
  tracing->StopTracing(content::TracingController::CreateStringEndpoint(
      base::BindOnce([](std::unique_ptr<std::string> trace) {
        const bc_host& host = browser_core::Host();
        if (host.trace_recorded) {
          host.trace_recorded(host.context, trace ? trace->data() : nullptr,
                              trace ? trace->size() : 0);
        }
      })));
}

// Probe for a warm Core on Linux run; never merged.
