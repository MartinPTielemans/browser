//! An element painted smaller or larger than it's laid out, as a layer's transform would.

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Point, Window,
};

/// `child`, painted at `factor` its size about `origin`, in the window's coordinates. It's laid
/// out and hit where it would be at full size.
pub fn scaled(child: impl IntoElement, factor: f32, origin: Point<Pixels>) -> Scaled {
    Scaled { child: child.into_any_element(), factor, origin }
}

pub struct Scaled {
    child: AnyElement,
    factor: f32,
    origin: Point<Pixels>,
}

impl IntoElement for Scaled {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Scaled {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.factor == 1. {
            return self.child.paint(window, cx);
        }
        window.with_scale(self.factor, self.origin, |window| self.child.paint(window, cx));
    }
}
