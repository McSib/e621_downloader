///
/// List and scroll terminal-menu items explained.
///

fn main() {
    use terminal_menu::{menu, label, button, list, scroll, run, mut_menu};
    let menu = menu(vec![
        label("lists and scrolls"),

        // with list and scroll you can select a value from a group of values
        // you can change the selected value with arrow keys, wasd, or enter

        // use arrow keys or wasd
        // enter to select

        // list:
        //  show all values
        //  surround the selected value with brackets
        list("li", vec!["Alice", "Bob", "Charlie"]),

        // scroll:
        //  show only the selected item
        scroll("sc", vec!["Alice", "Bob", "Charlie"]),

        button("exit")
    ]);
    run(&menu);
    {
        let mm = mut_menu(&menu);
        println!("{}", mm.selection_value("li"));
        println!("{}", mm.selection_value("sc"));
    }
}