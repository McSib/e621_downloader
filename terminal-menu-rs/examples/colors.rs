///
/// Add colors to your terminal-menu items.
///

fn main() {
    use terminal_menu::*;

    // see the crossterm crate for all the color options
    use crossterm::style::Color;

    let menu = menu(vec![


        label("White"),
        label("Red").colorize(Color::Red),
        label("Green").colorize(Color::Green),
        label("Blue").colorize(Color::Blue),

        // selected item is always cyan
        button("Cyan")
    ]);
    run(&menu);
}