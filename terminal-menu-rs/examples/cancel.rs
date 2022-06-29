///
/// Explains how menus are cancelled and how to detect cancellation.
///

fn main() {
    use terminal_menu::{menu, button, run, mut_menu};
    let menu = menu(vec![
        button("button")
    ]);
    run(&menu);

    // true if exited with 'q' or esc, false if button was pressed
    println!("{}", mut_menu(&menu).canceled());
}