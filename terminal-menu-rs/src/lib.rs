//! Display simple menus on the terminal!
//! [Examples](https://gitlab.com/xamn/terminal-menu-rs/tree/master/examples)

mod fancy_menu;
mod utils;

use std::sync::{
    Arc,
    RwLock,
    RwLockWriteGuard,
};
use std::thread;
use std::time::Duration;

use crossterm::style::Color;

pub type TerminalMenu = Arc<RwLock<TerminalMenuStruct>>;

enum TMIKind {
    Label,
    Button,
    BackButton,
    Scroll {
        values: Vec<String>,
        selected: usize,
    },
    List {
        values: Vec<String>,
        selected: usize,
    },
    String {
        value: String,
        allow_empty: bool,
    },
    Numeric {
        value: f64,
        step: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
    },
    Submenu(TerminalMenu),
}
pub struct TerminalMenuItem {
    name: String,
    kind: TMIKind,
    color: crossterm::style::Color,
}

/// Make a label terminal-menu item.
/// Can't be selected.
/// Useful for example as a title, separator, or help text.
/// # Example
/// ```
/// use terminal_menu::{menu, label, list};
/// let menu = menu(vec![
///     label("This is my menu:"),
///     list("This is my menu items name", vec!["foo", "bar", "baz"])
/// ]);
/// ```
pub fn label<T: Into<String>>(text: T) -> TerminalMenuItem {
    TerminalMenuItem {
        name: text.into(),
        kind: TMIKind::Label,
        color: Color::Reset,
    }
}

/// Make a button terminal-menu item.
/// Exits the menu with all the parent menus when pressed.
/// # Example
/// ```
/// use terminal_menu::{menu, button, run, mut_menu};
/// let my_menu = menu(vec![
///     button("Alice"),
///     button("Bob")
/// ]);
/// run(&my_menu);
/// println!("Selected Button: {}", mut_menu(&my_menu).selected_item_name());
/// ```
pub fn button<T: Into<String>>(name: T) -> TerminalMenuItem {
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::Button,
        color: Color::Reset,
    }
}

/// Make a back button terminal-menu item.
/// Returns to the previous menu (or exits when there is none) when pressed.
/// # Example
/// ```
/// use terminal_menu::{menu, back_button, submenu};
/// let menu = menu(vec![
///     submenu("Submenus Name", vec![
///         back_button("Back")
///     ]),
///     back_button("Exit"),
/// ]);
/// ```
pub fn back_button<T: Into<String>>(name: T) -> TerminalMenuItem {
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::BackButton,
        color: Color::Reset,
    }
}

/// Make a terminal-menu item from which you can select a value from a selection.
/// All values are dispalyed all the time.
/// # Example
/// ```
/// use terminal_menu::{menu, scroll, run, mut_menu};
/// let menu = menu(vec![
///     scroll("My Scrolls Name", vec![
///         "First Option",
///         "Second Option",
///         "Third Option"
///     ])
/// ]);
/// run(&menu);
/// println!("My Scrolls Value: {}", mut_menu(&menu).selection_value("My Scrolls Name"));
/// ```
pub fn scroll<T: Into<String>, T2: IntoIterator>(name: T, values: T2) -> TerminalMenuItem
where
    T2::Item: Into<String>,
{
    let values: Vec<String> = values.into_iter().map(|a| a.into()).collect();
    if values.is_empty() {
        panic!("values cannot be empty");
    }
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::Scroll {
            values,
            selected: 0,
        },
        color: Color::Reset,
    }
}

pub fn scroll_with_default_value<T: Into<String>, T2: IntoIterator>(
    name: T,
    values: T2,
    default_selection: usize,
) -> TerminalMenuItem
where
    T2::Item: Into<String>,
{
    let values: Vec<String> = values.into_iter().map(|a| a.into()).collect();
    if values.is_empty() {
        panic!("values cannot be empty");
    }
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::Scroll {
            values,
            selected: default_selection,
        },
        color: Color::Reset,
    }
}

/// Make a terminal-menu item from which you can select a value from a selection.
/// Only the selected value is visible.
/// # Example
/// ```
/// use terminal_menu::{menu, list, run, mut_menu};
/// let menu = menu(vec![
///     list("My Lists Name", vec![
///         "First Option",
///         "Second Option",
///         "Third Option"
///     ])
/// ]);
/// run(&menu);
/// println!("My Lists Value: {}", mut_menu(&menu).selection_value("My Lists Name"));
/// ```
pub fn list<T: Into<String>, T2: IntoIterator>(name: T, values: T2) -> TerminalMenuItem
where
    T2::Item: Into<String>,
{
    let values: Vec<String> = values.into_iter().map(|a| a.into()).collect();
    if values.is_empty() {
        panic!("values cannot be empty");
    }
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::List {
            values,
            selected: 0,
        },
        color: Color::Reset,
    }
}

pub fn list_with_default_value<T: Into<String>, T2: IntoIterator>(
    name: T,
    values: T2,
    default_selection: usize,
) -> TerminalMenuItem
where
    T2::Item: Into<String>,
{
    let values: Vec<String> = values.into_iter().map(|a| a.into()).collect();
    if values.is_empty() {
        panic!("values cannot be empty");
    }
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::List {
            values,
            selected: default_selection,
        },
        color: Color::Reset,
    }
}

/// Make a terminal-menu item which you can enter a string of characters to.
/// Empty strings may be enabled with a flag.
/// # Example
/// ```
/// use terminal_menu::{menu, string, run, mut_menu};
/// let menu = menu(vec![
///     string("My Strings Name", "Default Value", /* allow empty string */ false)
/// ]);
/// run(&menu);
/// println!("My Strings Value: {}", mut_menu(&menu).selection_value("My Strings Name"));
/// ```
pub fn string<T: Into<String>, T2: Into<String>>(
    name: T,
    default: T2,
    allow_empty: bool,
) -> TerminalMenuItem {
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::String {
            value: default.into(),
            allow_empty,
        },
        color: Color::Reset,
    }
}

/// Make a terminal-menu item from which you can select a number between specified bounds.
/// # Example
/// ```
/// use terminal_menu::{menu, numeric, run, mut_menu};
/// let menu = menu(vec![
///     numeric("My Numerics Name",
///         0.0,  //default
///         Some(0.5),  //step (optional)
///         Some(-5.0), //minimum (optional)
///         Some(10.0)  //maximum (optional)
///     )
/// ]);
/// run(&menu);
/// println!("My Numerics Value: {}", mut_menu(&menu).numeric_value("My Numerics Name"))
/// ```
pub fn numeric<T: Into<String>>(
    name: T,
    default: f64,
    step: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
) -> TerminalMenuItem {
    if !utils::value_valid(default, step, min, max) {
        panic!("invalid default value");
    }
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::Numeric {
            value: default,
            step,
            min,
            max,
        },
        color: Color::Reset,
    }
}

/// Make a terminal-menu submenu item.
/// It is basically a menu inside a menu.
/// # Example
/// ```
/// use terminal_menu::{menu, submenu, list, button, back_button, run, mut_menu};
/// let menu = menu(vec![
///     submenu("My Submenus Name", vec![
///         list("List", vec!["First", "Second", "Third"]),
///         back_button("Back"),
///         button("Exit")
///     ]),
/// ]);
/// run(&menu);
/// println!("{}",
///     mut_menu(&menu)
///     .get_submenu("My Submenus Name")
///     .selection_value("List"));
/// ```
pub fn submenu<T: Into<String> + Clone>(name: T, items: Vec<TerminalMenuItem>) -> TerminalMenuItem {
    let menu = menu(items);
    menu.write().unwrap().name = Some(name.clone().into());
    TerminalMenuItem {
        name: name.into(),
        kind: TMIKind::Submenu(menu),
        color: Color::Reset,
    }
}

impl TerminalMenuItem {
    /// Get the name of the terminal-menu item.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set a color to print the item in.
    /// # Example
    /// ```
    /// use terminal_menu::{menu, label, scroll};
    /// use crossterm::style::Color;
    /// let menu = menu(vec![
    ///     label("Colorize me").colorize(Color::Magenta),
    ///     scroll("Me too!", vec!["foo", "bar"]).colorize(Color::Green)
    /// ]);
    /// ```
    pub fn colorize(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

pub(crate) enum PrintState {
    None,
    Small,
    Big,
}

pub struct TerminalMenuStruct {
    name: Option<String>,
    pub items: Vec<TerminalMenuItem>,
    selected: usize,
    active: bool,
    exited: bool,

    longest_name: usize,
    exit: Option<String>,
    canceled: bool,
    printed: PrintState,
}
impl TerminalMenuStruct {
    /// Returns the name of the selected menu item.
    /// # Example
    /// ```
    /// use terminal_menu::{menu, button, run, mut_menu};
    /// let my_menu = menu(vec![
    ///     button("a"),
    ///     button("b"),
    /// ]);
    /// run(&my_menu);
    /// println!("selected item name: {}", mut_menu(&my_menu).selected_item_name()); //"a" or "b"
    /// ```
    pub fn selected_item_name(&self) -> &str {
        &self.items[self.selected].name
    }

    /// Returns the selected item as an index of the items vec.
    /// # Example
    /// ```
    /// use terminal_menu::{menu, button, run, mut_menu};
    /// let my_menu = menu(vec![
    ///     button("a"),
    ///     button("b"),
    /// ]);
    /// run(&my_menu);
    /// println!("selected item index: {}", mut_menu(&my_menu).selected_item_index()); // 0 or 1
    /// ```
    pub fn selected_item_index(&self) -> usize {
        self.selected
    }

    fn index_of(&self, name: &str) -> usize {
        self.items
            .iter()
            .position(|a| a.name == name)
            .expect("No item with the given name")
    }

    /// Set the selected item with a name.
    /// # Example
    /// ```
    /// use terminal_menu::{TerminalMenu, menu, button, mut_menu};
    /// let my_menu: TerminalMenu = menu(vec![
    ///     button("item"),
    ///     button("other item")
    /// ]);
    /// mut_menu(&my_menu).set_selected_item_with_name("item");
    /// ```
    pub fn set_selected_item_with_name(&mut self, item: &str) {
        self.selected = self.index_of(item);
    }

    /// Set the selected item with an index of the items vec.
    /// # Example
    /// ```
    /// use terminal_menu::{TerminalMenu, menu, button, mut_menu};
    /// let my_menu: TerminalMenu = menu(vec![
    ///     button("item"),
    ///     button("other item")
    /// ]);
    /// mut_menu(&my_menu).set_selected_item_with_index(1); //index 1 = other item
    /// ```
    pub fn set_selected_item_with_index(&mut self, item: usize) {
        if item >= self.items.len() {
            panic!("index out of bounds");
        }
        self.selected = item;
    }

    /// Returns the value of the specified scroll, list, or string item.
    /// # Example
    /// ```
    /// use terminal_menu::{TerminalMenu, menu, scroll, run, mut_menu};
    /// let my_menu: TerminalMenu = menu(vec![
    ///     scroll("item", vec!["val1", "val2"])
    /// ]);
    /// run(&my_menu);
    /// println!("item value: {}", mut_menu(&my_menu).selection_value("item"));
    /// ```
    pub fn selection_value(&self, name: &str) -> &str {
        match &self.items[self.index_of(name)].kind {
            TMIKind::Scroll { values, selected } | TMIKind::List { values, selected } => {
                &values[*selected]
            }
            TMIKind::String { value, .. } => value,
            _ => panic!("item wrong kind"),
        }
    }

    /// Returns the value of the specified numeric item.
    /// # Example
    /// ```
    /// use terminal_menu::{TerminalMenu, menu, scroll, run, numeric, mut_menu};
    /// let my_menu: TerminalMenu = menu(vec![
    ///     numeric("item", 0.0, None, None, None)
    /// ]);
    /// run(&my_menu);
    /// println!("item value: {}", mut_menu(&my_menu).numeric_value("item"));
    /// ```
    pub fn numeric_value(&self, name: &str) -> f64 {
        match self.items[self.index_of(name)].kind {
            TMIKind::Numeric { value, .. } => value,
            _ => panic!("item wrong kind"),
        }
    }

    /// Returns the specified submenu.
    /// # Example
    /// ```
    /// use terminal_menu::{TerminalMenu, menu, run, submenu, scroll, mut_menu};
    /// let my_menu: TerminalMenu = menu(vec![
    ///     submenu("sub",vec![
    ///         scroll("item", vec!["winnie", "the", "pooh"])
    ///     ])
    /// ]);
    /// run(&my_menu);
    /// println!("{}", mut_menu(&my_menu).get_submenu("sub").selection_value("item"));
    /// ```
    pub fn get_submenu(&mut self, name: &str) -> RwLockWriteGuard<TerminalMenuStruct> {
        for item in &self.items {
            if item.name == name {
                if let TMIKind::Submenu(submenu) = &item.kind {
                    return submenu.write().unwrap();
                }
            }
        }
        panic!("Item not found or is wrong kind");
    }

    /// Returns the menu (or submenu) which was active on deactivation.
    pub fn get_latest_menu_name(&mut self) -> Option<&str> {
        match &self.exit {
            None => None,
            Some(a) => Some(a),
        }
    }

    /// Returns true if menu was exited with 'q' or esc
    /// # Example
    /// ```
    /// use terminal_menu::{menu, button, run, mut_menu};
    /// let menu = menu(vec![
    ///     button("button")
    /// ]);
    /// run(&menu);
    ///
    /// // true if esc, false if button
    /// println!("{}", mut_menu(&menu).canceled());
    /// ```
    pub fn canceled(&self) -> bool {
        self.canceled
    }
}

/// Create a terminal-menu. See the examples for more.
/// # Example
/// ```
/// use terminal_menu::*;
/// let my_menu = menu(vec![
///     label("label"),
///     button("button"),
///     scroll("scroll", vec!["a", "b", "c"])
/// ]);
/// run(&my_menu);
/// {
///     let mm = mut_menu(&my_menu);
///     println!("{}", mm.selection_value("scroll"));
///     println!("{}", mm.selected_item_name());
/// }
/// ```
pub fn menu(items: Vec<TerminalMenuItem>) -> TerminalMenu {
    for i in 0..items.len() {
        if let TMIKind::Label = items[i].kind {
        } else {
            return Arc::new(RwLock::new(TerminalMenuStruct {
                name: None,
                items,
                selected: i,
                active: false,
                exited: true,

                longest_name: 0,
                exit: None,
                canceled: false,
                printed: PrintState::None,
            }));
        }
    }
    panic!("no selectable items");
}

/// Returns true if the menu is inactive and has exited.
/// # Example
/// ```
/// use terminal_menu::{menu, numeric, string, run, activate, has_exited, mut_menu};
/// let mut my_menu = menu(vec![
///     numeric("Charlie", 46.5, None, Some(32332.2), None)
/// ]);
/// activate(&my_menu);
///
/// //stuff
///
/// if has_exited(&my_menu) {
///     let mut mutable_menu = mut_menu(&my_menu);
///     println!("Selected Item: {}", mutable_menu.selected_item_name());
///     mutable_menu.items.push(string("new item", "def", false));
/// }
///
/// run(&my_menu);
/// ```
pub fn has_exited(menu: &TerminalMenu) -> bool {
    menu.read().unwrap().exited
}

/// Get a mutable instance of the menu.
/// Works only if has_exited(&menu) is true.
/// # Example
/// ```
/// use terminal_menu::{menu, numeric, string, run, activate, has_exited, mut_menu};
/// let mut my_menu = menu(vec![
///     numeric("Charlie", 46.5, None, Some(32332.2), None)
/// ]);
/// activate(&my_menu);
///
/// //stuff
///
/// if has_exited(&my_menu) {
///     let mut mutable_menu = mut_menu(&my_menu);
///     println!("Selected Item: {}", mutable_menu.selected_item_name());
///     mutable_menu.items.push(string("new item", "def", false));
/// }
///
/// run(&my_menu);
/// ```
pub fn mut_menu(menu: &TerminalMenu) -> RwLockWriteGuard<TerminalMenuStruct> {
    if !has_exited(menu) {
        panic!("Cannot call mutable_instance if has_exited() is not true");
    }
    menu.write().unwrap()
}

/// Activate (open) the menu.
/// Menu will deactivate when deactivated manually or button items are pressed.
/// # Example
/// ```
/// use terminal_menu::{menu, numeric, string, run, activate, has_exited, mut_menu};
/// let mut my_menu = menu(vec![
///     numeric("Charlie", 46.5, None, Some(32332.2), None)
/// ]);
/// activate(&my_menu);
///
/// //stuff
///
/// if has_exited(&my_menu) {
///     let mut mutable_menu = mut_menu(&my_menu);
///     println!("Selected Item: {}", mutable_menu.selected_item_name());
///     mutable_menu.items.push(string("new item", "def", false));
/// }
///
/// run(&my_menu);
/// ```
pub fn activate(menu: &TerminalMenu) {
    let menu = menu.clone();
    thread::spawn(move || fancy_menu::run(menu.clone()));
}

/// Deactivate (exit) a menu manually.
/// # Example
/// ```
/// use terminal_menu::{menu, numeric, activate, deactivate};
/// let my_menu = menu(vec![
///     list("galadriel", vec!["frodo", "bilbo"])
///     numeric("boo", 4.67, Some(3.0), None, None)
/// ]);
/// activate(&my_menu);
///
/// //do something here
///
/// deactivate(&my_menu);
/// ```
pub fn deactivate(menu: &TerminalMenu) {
    menu.write().unwrap().active = false;
    wait_for_exit(menu);
}

/// Wait for menu to exit.
/// # Example
/// ```
/// use terminal_menu::{TerminalMenu, menu, activate, deactivate, wait_for_exit};
/// let my_menu = menu(vec![
///     list("galadriel", vec!["frodo", "bilbo"])
///     numeric("boo", 4.67, Some(3.0), None, None)
/// ]);
/// activate(&my_menu);
///
/// //do something here
///
/// wait_for_exit(&my_menu);
///```
pub fn wait_for_exit(menu: &TerminalMenu) {
    loop {
        thread::sleep(Duration::from_millis(10));
        if has_exited(menu) {
            break;
        }
    }
}

/// Activate the menu and wait for it to exit.
/// # Example
/// ```
/// use terminal_menu::{TerminalMenu, menu, run};
/// let my_menu = menu(vec![
///     list("galadriel", vec!["frodo", "bilbo"])
///     numeric("boo", 4.67, Some(3.0), None, None)
/// ]);
/// run(&my_menu);
/// ```
pub fn run(menu: &TerminalMenu) {
    fancy_menu::run(menu.clone());
}
