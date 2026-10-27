// A page shown by the system's own web view in place of Chromium's: WebKit's on the Mac, and later
// WebView2 on Windows. A tab switches to one for a site whose protected video only the system's
// engine plays (docs/decisions.md, decision 1). The tab keeps its id, its Space and its place;
// only its pane's view and what it reports come from here.

#ifndef BROWSER_CORE_PLATFORM_SYSTEM_PAGE_H_
#define BROWSER_CORE_PLATFORM_SYSTEM_PAGE_H_

#include <memory>
#include <string>

#include "base/functional/callback_forward.h"
#include "ui/gfx/native_ui_types.h"
#include "url/gurl.h"

namespace input {
struct NativeWebKeyboardEvent;
}

namespace browser_core::platform {

// Where a system page keeps its cookies and site data: one store a profile, apart from Chromium's
// and from Safari's. |id| is a UUID for a profile's, or for a private session's, whose store keeps
// nothing on disk and goes once ReleaseStore lets it go.
struct SystemStore {
  std::string id;
  bool persistent;
};

class SystemPage {
 public:
  // What the page tells its tab, on the main thread. SystemPageChanged can come inside the tab's
  // own calls, as WebKit's observers do; the rest come from tasks of their own.
  class Delegate {
   public:
    // Its URL, title, loading, history or sound changed.
    virtual void SystemPageChanged() = 0;
    // A new document committed in its main frame.
    virtual void SystemPageCommitted() = 0;
    // That document finished loading.
    virtual void SystemPageLoaded() = 0;
    // The user followed a link to |url|, on another site, in the page's main frame. The page
    // doesn't load it: the tab loads it, here or in Chromium.
    virtual void SystemPageLeft(const GURL& url) = 0;
    // The page asked for |url| in a new window, as a link with a target or window.open do.
    virtual void SystemPageOpen(const GURL& url) = 0;
    // A key going down with the page focused, before the page sees it, or once the page didn't
    // keep it. |editing| if the page's focused element takes text. Returns whether the app used
    // it.
    virtual bool SystemPageKey(const input::NativeWebKeyboardEvent& event,
                               bool before_page,
                               bool editing) = 0;
    // A click in the page.
    virtual void SystemPageClicked() = 0;

   protected:
    virtual ~Delegate() = default;
  };

  // Whether the platform has a system engine that keeps a store per profile: macOS 14 or later.
  // Answers without loading it.
  static bool Available();
  // A page loading |url| in |store|, with the system's engine loaded the first time. Null where
  // there's none.
  static std::unique_ptr<SystemPage> Create(const SystemStore& store,
                                            const GURL& url,
                                            Delegate* delegate);
  // Deletes |id|'s persistent store from the disk, as its profile goes. Loads the system's engine:
  // call it only for a store a page made.
  static void DeleteStore(const std::string& id);
  // Lets a private session's store go. Never loads the system's engine.
  static void ReleaseStore(const std::string& id);
  // Deletes |domain|'s cookies, or else its cache, from |store|, then runs |done|. Loads the
  // system's engine, as DeleteStore.
  static void ClearSite(const SystemStore& store,
                        const std::string& domain,
                        bool cookies,
                        base::OnceClosure done);

  SystemPage(const SystemPage&) = delete;
  SystemPage& operator=(const SystemPage&) = delete;
  virtual ~SystemPage() = default;

  // The view the window shows in the tab's pane, as it shows a WebContents's.
  virtual gfx::NativeView view() = 0;
  // Gives the page the keyboard.
  virtual void Focus() = 0;

  virtual GURL url() = 0;
  virtual std::u16string title() = 0;
  virtual bool loading() = 0;
  virtual bool can_go_back() = 0;
  virtual bool can_go_forward() = 0;
  // Its connection is https and everything it loaded came over it.
  virtual bool secure() = 0;
  virtual bool audible() = 0;
  virtual bool muted() = 0;

  virtual void Load(const GURL& url) = 0;
  virtual void GoBack() = 0;
  virtual void GoForward() = 0;
  virtual void Reload(bool bypass_cache) = 0;
  virtual void Stop() = 0;
  virtual void SetMuted(bool muted) = 0;

 protected:
  SystemPage() = default;
};

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_SYSTEM_PAGE_H_
