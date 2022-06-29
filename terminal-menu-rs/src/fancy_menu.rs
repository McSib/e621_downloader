use std::io::{stdout, Write, stdin};
use crate::{TerminalMenu, TerminalMenuStruct, TMIKind, utils, back_button, PrintState};
use crossterm::*;

pub fn run(menu: TerminalMenu) {
    {
        let mut menu_wr = menu.write().unwrap();
        menu_wr.active = true;
        menu_wr.exited = false;
        menu_wr.canceled = false;

        menu_wr.longest_name = menu_wr.items.iter().map(|a| a.name.len()).max().unwrap();

        print(&mut menu_wr);
    }

    terminal::enable_raw_mode().unwrap();
    execute!(
        stdout(),
        cursor::Hide
    ).unwrap();

    while menu.read().unwrap().active {
        handle_input(&menu);
    }

    terminal::disable_raw_mode().unwrap();
    execute!(
        stdout(),
        cursor::Show,
    ).unwrap();

    {
        let mut menu_wr = menu.write().unwrap();
        if let PrintState::Small = menu_wr.printed {
            utils::unprint(menu_wr.items.len());
        } else if let PrintState::Big = menu_wr.printed {
            execute!(
                stdout(),
                terminal::LeaveAlternateScreen
            ).unwrap();
        }
        menu_wr.printed = PrintState::None;
        menu_wr.exited = true;
    }
}

fn print(menu_wr: &mut TerminalMenuStruct) {
    if let PrintState::Big = menu_wr.printed {
        print_big(menu_wr);
    } else if menu_wr.items.len() + 1 >= utils::term_height() || cfg!(windows) {
        print_big(menu_wr);
    } else if let PrintState::None = menu_wr.printed {
        for i in 0..menu_wr.items.len() {
            print_item(&menu_wr, i);
            println!();
        }
        menu_wr.printed = PrintState::Small;
    }
}

fn print_big(menu: &mut TerminalMenuStruct) {
    let term_height = utils::term_height();
    if term_height <= 3 {
        return;
    }
    if let PrintState::Small = menu.printed {
        utils::unprint(menu.items.len());
    }
    if let PrintState::Small | PrintState::None = menu.printed {
        queue!(
            stdout(),
            terminal::EnterAlternateScreen
        ).unwrap();
    }
    queue!(
        stdout(),
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All),
        style::Print("..."),
    ).unwrap();
    println!("\r");

    let item_count = menu.items.len().min(term_height - 3);
    let mut top = 0;
    if menu.selected > item_count / 2 {
        top = menu.selected - item_count / 2;
        if top + item_count > menu.items.len() {
            top = menu.items.len() - item_count;
        }
    }
    for i in top..(top + item_count) {
        print_item(menu, i);
        println!("\r");
    }
    println!("...");
    menu.printed = PrintState::Big;
}

fn print_item(menu: &TerminalMenuStruct, index: usize) {
    if menu.selected == index {
        queue!(
            stdout(),
            crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
            crossterm::style::Print("> "),
            crossterm::style::Print(&menu.items[index].name),
        ).unwrap();
    } else {
        queue!(
            stdout(),
            crossterm::style::Print("  "),
            crossterm::style::SetForegroundColor(menu.items[index].color),
            crossterm::style::Print(&menu.items[index].name)
        ).unwrap();
    }

    for _ in menu.items[index].name.len()..menu.longest_name + 5 {
        queue!(
            stdout(),
            crossterm::style::Print(" ")
        ).unwrap();
    }

    match &menu.items[index].kind {
        TMIKind::Label      |
        TMIKind::Button     |
        TMIKind::BackButton |
        TMIKind::Submenu(_) => {}
        TMIKind::List { values, selected } => {
            for i in 0..values.len() {
                if i == *selected {
                    queue!(
                        stdout(),
                        style::Print("["),
                        style::Print(&values[i]),
                        style::Print("]")
                    ).unwrap();
                } else {
                    queue!(
                        stdout(),
                        style::Print(" "),
                        style::Print(&values[i]),
                        style::Print(" ")
                    ).unwrap();
                }
            }
        }
        TMIKind::Scroll { values, selected } => {
            queue!(
                stdout(),
                style::Print(" "),
                style::Print(values.iter().nth(*selected).unwrap()),
            ).unwrap();
        }
        TMIKind::String { value, .. } => {
            queue!(
                stdout(),
                style::Print(" "),
                style::Print(value)
            ).unwrap();
        }
        TMIKind::Numeric { value, .. } => {
            queue!(
                stdout(),
                style::Print(" "),
                style::Print(value)
            ).unwrap()
        }
    }

    queue!(
        stdout(),
        style::ResetColor
    ).unwrap();

}

fn handle_input(menu: &TerminalMenu) {
    while crossterm::event::poll(*utils::INTERVAL).unwrap() {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => {
                let mut menu_wr = menu.write().unwrap();
                let selected = menu_wr.selected;
                use crossterm::event::KeyCode::*;
                match key_event.code {
                    Up    | Char('w') | Char('k') => {
                        let new = dec(&menu_wr, selected);
                        select(&mut menu_wr, new);
                    },
                    Down  | Char('s') | Char('j') => {
                        let new = inc(&menu_wr, selected);
                        select(&mut menu_wr, new);
                    },
                    Left  | Char('a') | Char('h') => dec_value(&mut menu_wr),
                    Right | Char('d') | Char('l') => inc_value(&mut menu_wr),
                    Enter | Char(' ') => handle_enter(&mut menu_wr),
                    Esc   | Char('q') => {
                        menu_wr.active = false;
                        menu_wr.exit = menu_wr.name.clone();
                        menu_wr.canceled = true;
                        return;
                    },
                    _ => {}
                }
            }
            event::Event::Resize(_, _) => {
                print(&mut menu.write().unwrap());
            }
            _ => {}
        }
    }
}

fn print_in_place(menu: &TerminalMenuStruct, index: usize) {
    queue!(
        stdout(),
        cursor::SavePosition,
        cursor::MoveUp((menu.items.len() - index) as u16),
        terminal::Clear(terminal::ClearType::UntilNewLine)
    ).unwrap();
    print_item(menu, index);
    queue!(
        stdout(),
        cursor::RestorePosition
    ).unwrap();
}

fn select(menu: &mut TerminalMenuStruct, index: usize) {
    let old_active = menu.selected;
    menu.selected = index;
    if let PrintState::Small = menu.printed {
        print_in_place(menu, old_active);
        print_in_place(menu, index);
    } else {
        print_big(menu);
    }
    stdout().flush().unwrap();
}

fn inc(menu: &TerminalMenuStruct, mut index: usize) -> usize {
    index += 1;
    if index == menu.items.len() {
        index = 0;
    }
    if let TMIKind::Label = menu.items[index].kind {
        inc(menu, index)
    } else {
        index
    }
}

fn dec(menu: &TerminalMenuStruct, mut index: usize) -> usize {
    if index == 0 {
         index = menu.items.len() - 1;
    } else {
        index -= 1
    }
    if let TMIKind::Label = menu.items[index].kind {
        dec(menu, index)
    } else {
        index
    }
}

fn handle_enter(menu: &mut TerminalMenuStruct) {
    let item_count = menu.items.len();
    match &mut menu.items[menu.selected].kind {
        TMIKind::Button => {
            menu.exit = menu.name.clone();
            menu.active = false;
        }
        TMIKind::BackButton => {
            menu.active = false;
        }
        TMIKind::Scroll { selected, values } |
        TMIKind::List { selected, values } => {
            let temp_menu =
                crate::menu(values.iter().enumerate().map(|(i, s)|
                    if i == *selected {
                        back_button(s).colorize(style::Color::Green)
                    } else {
                        back_button(s)
                    }
                ).collect());
            temp_menu.write().unwrap().selected = *selected;
            if let PrintState::Small = menu.printed {
                terminal::disable_raw_mode().unwrap();
                utils::unprint(item_count);
                menu.printed = PrintState::None;
            }

            if let PrintState::Big = menu.printed {
                temp_menu.write().unwrap().printed = PrintState::Big;
            }

            crate::run(&temp_menu);

            *selected = temp_menu.read().unwrap().selected;

            menu.printed = PrintState::None;
            print(menu);
            terminal::enable_raw_mode().unwrap();
            execute!(
                stdout(),
                cursor::Hide
            ).unwrap();
        }
        TMIKind::String { value, allow_empty } => {
            if let PrintState::Big = menu.printed {
                queue!(
                    stdout(),
                    cursor::MoveToNextLine(100)
                ).unwrap();
            }
            print!(": ");
            stdout().flush().unwrap();
            terminal::disable_raw_mode().unwrap();
            execute!(
                stdout(),
                cursor::Show,
            ).unwrap();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            input = input.trim().to_owned();
            terminal::enable_raw_mode().unwrap();
            execute!(
                stdout(),
                cursor::Hide,
            ).unwrap();
            utils::unprint(1);
            if *allow_empty || !input.is_empty() {
                *value = input;
            }
            if let PrintState::Big = menu.printed  {
                print(menu);
            } else {
                print_in_place(menu, menu.selected);
                stdout().flush().unwrap();
            }
        }
        TMIKind::Numeric { value, step, min, max } => {
            if let PrintState::Big = menu.printed {
                queue!(
                    stdout(),
                    cursor::MoveToNextLine(100)
                ).unwrap();
            }
            utils::number_range_indicator(*step, *min, *max);
            stdout().flush().unwrap();
            terminal::disable_raw_mode().unwrap();
            execute!(
                stdout(),
                cursor::Show,
            ).unwrap();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            terminal::enable_raw_mode().unwrap();
            execute!(
                stdout(),
                cursor::Hide,
            ).unwrap();
            utils::unprint(1);
            if let Ok(input) = input.trim().parse() {
                if utils::value_valid(input, *step, *min, *max) {
                    *value = input;
                }
            }
            if let PrintState::Big = menu.printed  {
                print(menu);
            } else {
                print_in_place(menu, menu.selected);
                stdout().flush().unwrap();
            }
        }
        TMIKind::Submenu(submenu) => {
            if let PrintState::Small = menu.printed {
                terminal::disable_raw_mode().unwrap();
                utils::unprint(item_count);
                menu.printed = PrintState::None;
            }

            if let PrintState::Big = menu.printed {
                submenu.write().unwrap().printed = PrintState::Big;
            }

            crate::run(submenu);

            if let Some(exit_menu) = &submenu.clone().read().unwrap().exit {
                menu.exit = Some(exit_menu.clone());
                menu.canceled = submenu.read().unwrap().canceled;
                menu.active = false;
            } else {
                menu.printed = PrintState::None;
                print(menu);
                terminal::enable_raw_mode().unwrap();
                execute!(
                    stdout(),
                    cursor::Hide
                ).unwrap();
            }
        }
        _ => {}
    }
}

fn inc_value(menu: &mut TerminalMenuStruct) {
    match &mut menu.items[menu.selected].kind {
        TMIKind::Scroll { values, selected } |
        TMIKind::List   { values, selected }=> {
            *selected += 1;
            if *selected == values.len() {
                *selected = 0;
            }

        }
        TMIKind::Numeric { value, step, max, .. } => {
            if let Some(step) = step {
                *value += *step;
                if let Some(max) = max {
                    if *value > *max {
                        *value = *max;
                    }
                }
            }
        }
        _ => return
    }
    if let PrintState::Big = menu.printed {
        print(menu);
    } else {
        print_in_place(&menu, menu.selected);
        stdout().flush().unwrap();
    }
}

fn dec_value(menu: &mut TerminalMenuStruct) {
    match &mut menu.items[menu.selected].kind {
        TMIKind::Scroll { values, selected } |
        TMIKind::List   { values, selected }=> {
            if *selected == 0 {
                *selected = values.len() - 1;
            } else {
                *selected -= 1;
            }
        }
        TMIKind::Numeric { value, step, min, .. } => {
            if let Some(step) = step {
                *value -= *step;
                if let Some(min) = min {
                    if *value < *min {
                        *value = *min;
                    }
                }
            }
        }
        _ => return
    }
    if let PrintState::Big = menu.printed {
        print(menu);
    } else {
        print_in_place(&menu, menu.selected);
        stdout().flush().unwrap();
    }
}