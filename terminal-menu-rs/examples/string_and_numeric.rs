///
/// String and numeric terminal-menu items explained.
///

fn main() {
    use terminal_menu::{menu, label, button, string, numeric, run, mut_menu};
    let menu = menu(vec![
        label("strings and numerics"),

        // string:
        //  a string of characters
        //  the last arguments specifies if empty strings are allowed

        // empty strings allowed:
        string("ste", "default", true),

        // empty strings not allowed:
        string("stn", "default", false),

        // numeric:
        //  a floating point number
        numeric("num",
            // default
            4.5,

            // step
            Some(1.5),

            // minimum
            None,

            // maximum
            Some(150.0)
        ),

        button("exit")
    ]);
    run(&menu);
    {
        let mm = mut_menu(&menu);
        println!("{}", mm.selection_value("ste"));
        println!("{}", mm.selection_value("stn"));
        println!("{}", mm.numeric_value("num"));
    }
}