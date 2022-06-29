// use std::io::stdout;
//
// use crossterm::execute;
// use crossterm::terminal::{
//     Clear,
//     ClearType,
// };
// use inquire::Select;
//
// use crate::SubMenu;
//
// pub(crate) enum MenuOption<'a, T>
// where
//     T: Fn(),
// {
//     Label(&'a str, T),
//     SubMenu(Menu<'a>),
//     BackButton(&'a str),
// }
//
// pub(crate) struct Menu<'a> {
//     name: &'a str,
//     options: Vec<MenuOption<'a>>,
//     selected_option: usize,
// }
//
// impl<'a> Menu<'a> {
//     pub fn new(name: &'a str, options: Vec<MenuOption<'a>) -> Self {
//         if options.len() == 0 {
//             panic!("Menu must have at least one option.");
//         }
//
//         Menu {
//             name,
//             options,
//             selected_option: 0,
//         }
//     }
//
//     pub fn get_selected_option(&self) -> usize {
//         self.selected_option
//     }
//
//     pub fn select_option(&mut self, selection: &str) {
//         for (i, option) in self.options.iter().enumerate() {
//             match option {
//                 MenuOption::Label(label) | MenuOption::BackButton(label) => {
//                     if selection == *label {
//                         self.selected_option = i;
//                         break;
//                     }
//                 }
//                 MenuOption::SubMenu(menu) => {
//                     if selection == menu.name {
//                         self.selected_option = i;
//                         break;
//                     }
//                 }
//             }
//         }
//     }
//
//     pub fn get_selected_label(&self) -> Option<&str> {
//         if let MenuOption::Label(label) = &self.options[self.selected_option] {
//             Some(label)
//         } else {
//             None
//         }
//     }
//
//     pub fn is_back_button(&self) -> bool {
//         if let MenuOption::BackButton(_) = &self.options[self.selected_option] {
//             true
//         } else {
//             false
//         }
//     }
//
//     pub fn run(&mut self) {
//         // print out all the options, if it is a submenu, print out the submenu label as an available option.
//         let mut string_options = vec![];
//         for option in self.options.iter() {
//             match option {
//                 MenuOption::Label(label) | MenuOption::BackButton(label) => {
//                     string_options.push(label.to_string())
//                 }
//                 MenuOption::SubMenu(menu) => string_options.push(menu.name.to_string()),
//             }
//         }
//
//         let mut running = true;
//         while running {
//             let selection = Select::new(self.name, string_options.clone());
//             let selected_prompt = selection.prompt().unwrap();
//             self.select_option(&selected_prompt);
//
//             if let SubMenu(menu) = &mut self.options[self.selected_option] {
//                 menu.run();
//             }
//
//             execute!(stdout(), Clear(ClearType::All)).unwrap();
//
//             if self.is_back_button() {
//                 running = false;
//             }
//         }
//     }
// }
