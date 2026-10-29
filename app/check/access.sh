# VoiceOver reads ilo's own tree: the sidebar's Space name is a heading, and the tab shown a
# selected tab named by its page's title in the Today tab group, after New Tab. ⌘T's bar has the
# keyboard in a text field called "Search or Enter URL", lists its suggestions with the first
# selected, and ↓ selects the next; ⇧F10 opens its field's menu on the first item. A tab's menu is
# the app's window in front and a menu of menu items and separators, whose Move to opens a submenu. It reads the tree from outside, as an assistive app does,
# through tools/ax-tree.sh, which needs this terminal to have the Accessibility permission. The
# check VM may not have it: then the run says it skipped, and it never asks for it.

page='data:text/html,<title>Docs</title><p>A page for VoiceOver to find in the sidebar.'
log="$out/access.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* Docs$' || fail "the page didn't draw its title"
a=$(today)
# The lines deeper than the first line of tree $2 that has $1, up to the next as shallow.
under() {
  awk -v at="$1" 'found && $1 <= depth { exit } found { print }
    !found && index($0, at) { found = 1; depth = $1 }' "$2"
}

tree="$out/access-sidebar.txt"
../tools/ax-tree.sh "$pid" 'AXTabGroup/- "Today"' > "$tree"
if grep -q '^accessibility: skipped' "$tree"; then
  exec 3>&-
  quit
  echo "skip: access. This terminal doesn't have the Accessibility permission, so nothing read ilo's tree."
else
  grep -q ' AXHeading/- "[^"]*" id=space-name$' "$tree" ||
    fail "the Space's name isn't a heading: $tree"
  grep -q ' AXButton/- "New Tab".* id=new-tab$' "$tree" || fail "New Tab isn't a button: $tree"
  under 'AXTabGroup/- "Today"' "$tree" |
    grep -q " AXRadioButton/AXTabButton \"Docs\" value=1 id=tab-$a\$" ||
    fail "the tab shown isn't the selected tab Docs in Today: $tree"
  # New Tab comes before Today, as it's drawn.
  [ "$(grep -n 'id=new-tab$' "$tree" | cut -d: -f1)" -lt \
    "$(grep -n 'AXTabGroup/- "Today"' "$tree" | cut -d: -f1)" ] ||
    fail "New Tab doesn't come before Today: $tree"

  send click $page_point
  if [ "$active" != 1 ]; then
    exec 3>&-
    quit
    echo "skip: access's command bar. Browser wasn't the active app, so ⌘T would go elsewhere. The sidebar's tree: $tree"
  else
    send key t cmd
    expect '^browser: command bar shown$' "⌘T didn't open the command bar"
    send type docs
    expect '^browser: command bar suggestions "docs"' "the command bar didn't suggest for \"docs\""
    bar="$out/access-bar.txt"
    ../tools/ax-tree.sh "$pid" '"Suggestions"' > "$bar"
    grep -q ' AXTextField/- "Search or Enter URL" value=docs' "$bar" ||
      fail "the command bar's field isn't a text field with what was typed: $bar"
    rows=$(under '"Suggestions"' "$bar")
    [ "$(echo "$rows" | grep -c ' AXStaticText/')" -ge 2 ] ||
      fail "the command bar doesn't list two suggestions: $bar"
    echo "$rows" | grep ' AXStaticText/' | head -1 | grep -q ' selected' ||
      fail "the first suggestion isn't selected: $bar"
    send key down
    expect '^browser: command bar selected ' "↓ didn't select the next suggestion"
    moved="$out/access-bar-down.txt"
    ../tools/ax-tree.sh "$pid" '"Suggestions"' > "$moved"
    under '"Suggestions"' "$moved" | grep ' AXStaticText/' | sed -n 2p | grep -q ' selected' ||
      fail "↓ didn't move the selected suggestion: $moved"
    # ⇧F10, Full Keyboard Access's way to a menu, opens the field's under the caret on its first
    # item, and escape gives the keyboard back to the field.
    send key f10 shift
    expect '^browser: menu shown Cut[^,]*, Copy[^,]*, Paste, ' "⇧F10 didn't open the field's menu"
    expect '^browser: menu highlights \(Cut\|Paste\) at ' \
      "the field's menu from ⇧F10 didn't highlight its first item"
    expect '^browser: menu window at ' "the field's menu didn't take the keyboard"
    send key escape
    expect '^browser: menu closed$' "escape didn't close the field's menu"
    none_since '^browser: command bar hidden$' || fail "escape in the field's menu closed the bar"
    send key escape
    expect '^browser: command bar hidden$' "escape didn't close the command bar"

    # ilo's menu is the app's window in front, and reads as a menu: a menu item for each row, a
    # separator between groups, and Move to saying it opens a submenu, which → opens.
    post rightclick $(row 0)
    expect "^browser: menu shown $(cased 'Copy Link, ')" "a right-click on the tab didn't show its menu"
    expect '^browser: menu window at ' "the tab's menu didn't open its window"
    size=$(grep '^browser: menu window at' "$log" | tail -1 | cut -d' ' -f6)
    windows="$out/access-menu-windows.txt"
    ../tools/ax-windows.sh "$pid" > "$windows"
    sed -n '/^window server, front first:/{n;p;}' "$windows" | grep -q " $size\$" ||
      fail "the tab's menu, $size, isn't the app's window in front: $windows"
    menu="$out/access-menu.txt"
    ../tools/ax-tree.sh "$pid" 'AXMenuItem/' > "$menu"
    rows=$(under ' AXMenu/' "$menu")
    grep -q ' AXMenu/[^ ]* "Menu"' "$menu" || fail "the tab's menu isn't a menu: $menu"
    echo "$rows" | grep -q ' AXMenuItem/[^ ]* "Duplicate"' || fail "Duplicate isn't a menu item: $menu"
    echo "$rows" | grep -q ' AXSplitter/' || fail "the lines between groups aren't separators: $menu"
    echo "$rows" | grep -q ' AXMenuItem/[^ ]* "Move to" .*collapsed' ||
      fail "Move to doesn't say it opens a submenu: $menu"
    send key m
    expect '^browser: menu highlights Move to at ' "typing m didn't select Move to"
    send key right
    expect '^browser: menu submenu Move to shown at ' "→ didn't open Move to's submenu"
    submenu="$out/access-menu-submenu.txt"
    ../tools/ax-tree.sh "$pid" 'AXMenu/- "Move to"' > "$submenu"
    grep -q ' AXMenuItem/[^ ]* "Move to" .*expanded' "$submenu" ||
      fail "Move to doesn't say its submenu is open: $submenu"
    under ' AXMenu/- "Move to"' "$submenu" | grep -q ' AXMenuItem/[^ ]* "New Space…"' ||
      fail "Move to's submenu doesn't list New Space…: $submenu"
    send key escape
    send key escape
    expect '^browser: menu closed$' "escape didn't close the tab's menu"
    exec 3>&-
    quit
    echo "ok: VoiceOver reads the Space's name as a heading, the tab shown as the selected tab Docs in Today after New Tab, ⌘T's bar as a text field with its suggestions in a list whose selection ↓ moves, ⇧F10's field menu on its first item, and the tab's menu, in front, as a menu with its items, separators and a submenu → opens. Trees: $tree, $bar, $moved, $menu, $submenu, $windows"
  fi
fi
