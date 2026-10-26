# UX principles

[ux-spec.md](ux-spec.md) says what Arc does, measured. This file says why, as far as the measurements let us read it, so we can make the calls Arc's recordings don't cover and design what Arc never had, such as agents, the same way.

Each principle has what was **seen**, with the spec section it comes from, our **reading** of why, and how it **applies** to new work. The seen part is measured. The reading is ours, and a later recording can prove it wrong. When a new measurement fits a principle, add it to what was seen. When one contradicts a principle, rewrite the principle.

## Everything has a place, and motion shows where

- **Seen.** The Library sits left of the first Space, and New Space past the last: two fingers past either end reach them (Spaces › Switching). The Library pushes the page aside instead of covering it, and the sidebar under it shrinks toward the page as it goes (Motion › Library push, Library recede). The Space delete dialog drops 32 pt into place and leaves back up the same way. The Space form sinks as it leaves (Spaces › Creating). Back, forward and reload stay where they are while they can't act, dimmed to white 0.26 (Sidebar › Header and URL field).
- **Reading.** If you see where a thing came from, you know where to find it again. A place you remember replaces a menu you'd have to read.
- **Applies.** A new surface gets a place before it gets an animation. It comes in from that place and leaves the way it came. Only menus and popups at the pointer appear from nowhere.

## Under the fingers, physics; otherwise, a fixed time

- **Seen.** When the fingers lift, the Space pages settle on a spring that starts at the fingers' speed, ω ≈ 33 (Motion › Space settle). The sidebar reveal springs about 4 pt past its rest. A key or a click moves things on fixed curves instead: ⌘S hides the sidebar in 110 ms, and the delete dialog moves 0.275 s, ease-in-out.
- **Reading.** A dragged thing has to keep the speed the hand gave it, or it stops feeling held. A key press gives no speed to keep, so a fixed curve is the predictable choice.
- **Applies.** Whatever a gesture moves hands its speed to a spring when the gesture ends. Whatever a key or a click starts runs on a short fixed curve.

## One change at a time, in an order you can follow

- **Seen.** After Create, the Space form leaves from the bottom up: the buttons, the field 80 ms later, the title at 130 ms. The Space's rows show only once the title is gone (Spaces › Creating). The Library starts in only 0.29 of the way, so the sidebar is seen leaving first. After Delete, the Space switch waits until the dialog has gone, about 0.33 s after the click.
- **Reading.** Cause and effect stay readable. You see your action finish, then what it changed.
- **Applies.** When an action changes something elsewhere, such as a switch or a list, the thing acted on finishes leaving first. Staggered parts go in the order the eye meets them.

## Readable at once, settled a moment later

- **Seen.** The delete dialog is fully opaque after 0.135 s but moves for 0.275 s. Rows are within 0.75 pt of rest 83–92 ms after they start (Motion › Row motion). A click on the Library fades the sidebar out in 33 ms.
- **Reading.** Motion is there to explain, never to make you wait. You can read and act before it ends.
- **Applies.** Content is legible and takes input within about 130 ms. Motion can run past that, but never blocks a key or a click.

## A change shows that it happened

- **Seen.** A click on Copy URL turns its link into a check that draws itself from its left end, down, then up (Sidebar › URL field, the owner's look at Arc). Clear shows no toast: the rows leaving say what it did.
- **Reading.** A glyph that swaps in one frame is easy to miss, and a click that seems to do nothing gets clicked again. Motion on the thing clicked says it worked, where the eye already is, with nothing to read.
- **Applies.** A glyph that changes on the same thing moves into the new one (Motion › Glyph change). One that differs because something else is shown, such as another tab, shows at once: nothing changed, and a switch stays instant.

## Wait for intent before acting

- **Seen.** The reveal waits about 100 ms after the pointer reaches the window's edge, so a pointer passing on its way to another display leaves it shut. A swipe let go short of halfway goes back. The New Space pull makes a Space only if it's let go once readied, which a glow and a click on the trackpad announce (Spaces › Switching).
- **Reading.** An accidental trigger costs more than a short wait or a threshold.
- **Applies.** A gesture or a hover that opens something gets a threshold, and a visible, felt ready state before it commits.

## Quiet controls, loud thresholds

- **Seen.** Create Space doesn't change under the pointer or while pressed. The dialog's Delete darkens to 0.9 under the pointer. The dialog's buttons are labelled in macOS's control font, 13 pt regular, and only its question is heavier, at medium (Spaces › Renaming and deleting). The New Space pull pops to 1.13×, glows and clicks the trackpad as it readies, and clicks again if it un-readies. Arc's other haptic is for reordering tabs; it has none for swiping between Spaces (Settings › Advanced). Clear is the word alone until it would close the tab shown too, when an arrow comes before it (Clear divider).
- **Reading.** The strongest feedback goes where a gesture crosses a line. Everyday buttons stay calm, and the one that destroys something reacts. A button marks the press that would reach further than it looks, before it's pressed.
- **Applies.** Haptics, glows and overshoot are for threshold crossings. Every button still shows under the pointer that it can be clicked, quietly, in its kind's way: a plate, a fill, a step brighter or darker (the owner, 2026-10-26). Only one measured to stay still, such as Create Space, does. When a control's next press would take what's on screen, it says so before the press, quietly, on the control itself.

## Forgive rather than forbid

- **Seen.** Deleting a Space asks first, then archives its tabs instead of destroying them, and a toast offers ⌘Z. Clear asks nothing, shows no toast, and ⌘Z brings the tabs back. A closed Today tab goes to the archive. Cancel on a new Space deletes it and goes back to the Space it was made from.
- **Reading.** Confirmations train people to click through. Undo lets them act fast and recover.
- **Applies.** A destructive action gets an undo first. It asks as well only when a lot goes at once, and offers the undo anyway. One no undo can bring back, such as deleting a profile with its cookies and logins or removing an extension with its data, asks every time (ours).

## Teach the keys where they're used

- **Seen.** The delete dialog's buttons carry ESC and ↩ chips. The toast after a delete names ⌘Z.
- **Reading.** The keyboard path is learned at the moment it's useful, not from a help page.
- **Applies.** A button with a key shows the key on the button. A message that can be undone names the undo's key.

## A modifier changes where, not what

- **Seen.** ⌥↩ in the command bar opens its URL in a pane on the right (Split view › Creating a split). Dia puts the same ⌥ on "+ New Tab", and on a link with ⇧, since ⌥-click on a link downloads it; Chrome's is ⌥⌘-click. ⌘-click opens a link in a background tab and ⇧⌘-click in front (Keyboard).
- **Reading.** The action stays one you know, and the modifier picks where its result goes. That's one more key to learn, not one more command.
- **Applies.** A new destination for an existing action is a modifier on that action, not a new command. ⌥ puts the result beside the page. A modifier means the same in the command bar, the sidebar and the page.

## Delight on rare paths only

- **Seen.** Making a Space plays an illustration for about 2 s, and the pull glows. Switching, closing and clearing tabs have no flourish, and Clear has no toast.
- **Reading.** What you do a hundred times a day has to be instant. What you do once a month can be a moment.
- **Applies.** Decoration goes on rare paths. Hot paths stay plain and fast (the performance gate in [roadmap.md](roadmap.md)). We skip Arc's illustrations and their waits: the art is Arc's, and a 2 s wait isn't how we want it to feel.

## Pay when it's looked at

- **Seen.** A tab opened behind loads only once it's first shown, and until then its row shows the address and the site's icon (Tabs › Opening rules, Dia).
- **Reading.** Opening ten links to read later mustn't slow the page being read now. The cost moves to the moment someone looks, and the row still says what's waiting.
- **Applies.** Work for something not yet seen waits until it's seen, and its place shows what it will be. Work the user started on purpose, such as a form they sent, runs at once.

## Not known yet

Recordings could settle these. Until then, the calls listed in each PR stand.

- Whether Arc's dialog and form colours follow the Space's theme or are fixed.
- How Cancel looks under the pointer, and whether a click outside the dialog cancels it.
- What Arc does under Reduce Motion.
- How Arc's glyphs other than Copy URL's check change, such as reload into stop.
- Where Arc shows pinned extensions, and whether only under the pointer.
- How Arc asks about a page's permission request, such as for the camera: where it hangs, how it looks, and whether a hidden sidebar comes out for it. Ours hangs under the URL field, as Chrome's hangs under its address bar.
- What Arc does with more favorites than its sidebar has room for. Ours scroll with the list once they're taller than 40 % of the window, as Ardia's do.
