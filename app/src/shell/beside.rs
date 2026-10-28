//! The Spaces' pages as they page (docs/ux-spec.md › Spaces › Switching): the URL field and the
//! favorites are the page's, so the shown Space's move with its page, and the page of the Space
//! beside draws its own, as they show once it's shown.

use gpui::{AnyElement, div, prelude::*, px};

#[cfg(test)]
use gpui::canvas;

use super::{FOOTER_HEIGHT, Shell};

impl Shell {
    /// How far the shown Space's URL field and favorites are from their places: with its page
    /// while another Space's comes in beside it, else in place, as past the last Space, where its
    /// title and tabs rubber-band under them (Arc).
    pub(super) fn top_offset(&self) -> f32 {
        let pages = self.pages();
        match pages.len() {
            0 | 1 => 0.,
            _ => pages.iter().find(|&&(space, _)| space == self.space).map_or(0., |&(_, x)| x),
        }
    }

    /// `part`, the shown Space's URL field or favorites, with its page.
    pub(super) fn with_page(&self, part: impl IntoElement) -> AnyElement {
        let x = self.top_offset();
        if x == 0. {
            return part.into_any_element();
        }
        let page = div().absolute().top_0().bottom_0().left(px(x)).w(px(self.width())).child(part);
        div().absolute().size_full().overflow_hidden().child(page).into_any_element()
    }

    /// The pages of the Spaces beside the shown one's, while they page: each with its URL field,
    /// its favorites and its rows under them, as they show once it's shown, at its offset. They
    /// take no input.
    pub(super) fn beside_pages(&self) -> Vec<gpui::Div> {
        let width = self.width();
        let pages = self.pages().into_iter().filter(|&(space, _)| space != self.space);
        pages
            .filter_map(|(space, x)| {
                let space = self.spaces.iter().find(|other| other.id == space)?;
                let tab = self.browser.space_tab(space.id);
                let favorites = self.favorites_of(space);
                let rows = div()
                    .absolute()
                    .top(px(self.list_top_with(favorites.len())))
                    .left_0()
                    .w(px(width))
                    .bottom(px(FOOTER_HEIGHT))
                    .overflow_hidden()
                    .child(self.still_page(space, 0.));
                let page = div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(x))
                    .w(px(width))
                    .child(self.still_url_field(tab))
                    .child(rows)
                    .children(self.still_tiles(&favorites, tab));
                Some(div().absolute().size_full().overflow_hidden().child(page))
            })
            .collect()
    }

    /// Records where the page beside drew `tab`'s tile, or its URL field as 0, `outset` past its
    /// padding box (`framed`), for the tests.
    #[cfg(test)]
    pub(super) fn beside_framed(&self, tab: u64, outset: f32) -> impl IntoElement + use<> {
        let frames = self.beside_frames.clone();
        canvas(move |bounds, _, _| _ = frames.borrow_mut().insert(tab, bounds), |_, _, _, _| {})
            .absolute()
            .inset(px(-outset))
    }
}
