///
/// Terminal-menu submenus explained.
///

fn main() {
    use terminal_menu::{menu, label, button, scroll, back_button, submenu, run, mut_menu};
    let menu = menu(vec![
        label("submenus"),

        // submenu:
        //  submenus are basically menus inside menus
        submenu("sub", vec![
            scroll("scr", vec!["Alice", "Bob", "Charlie"]),

            // back button:
            //  back buttons return to the parent menu.
            back_button("back")
        ]),

        submenu("ret", vec![

            // button:
            //  buttons exit all the menus
            button("Alice"),
            button("Bob"),
            button("Charlie"),

        ]),

        button("exit")
    ]);
    run(&menu);

    // name of the menu active before exiting
    println!("{:?}", mut_menu(&menu).get_latest_menu_name());

    // pull values
    println!("{}", mut_menu(&menu).get_submenu("sub").selection_value("scr"));
}