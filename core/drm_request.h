#ifndef BROWSER_CORE_DRM_REQUEST_H_
#define BROWSER_CORE_DRM_REQUEST_H_

#include "content/public/browser/render_frame_host.h"
#include "mojo/public/cpp/bindings/binder_map.h"

namespace browser_core {

// g_browser_core_populate_frame_binders: lets a frame's page tell the core which protected-media
// key system it asks for. The core tells the app when it's one ilo can't play
// (tab_media_unsupported), once for each key system a document asks for, and takes it back once the
// tab goes to another site.
void PopulateFrameBinders(mojo::BinderMapWithContext<content::RenderFrameHost*>* map);

}  // namespace browser_core

#endif  // BROWSER_CORE_DRM_REQUEST_H_
