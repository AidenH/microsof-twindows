use std::process::Command;

use xcb::{x::{self, Window}, x::{EventMask, KeyButMask}};

// important program variables
struct State<'a> {
    con: &'a xcb::Connection,
    scr: &'a x::Screen,
    curr_win: Vec<Window>,
    item_list: Vec<WindowItem>,
    border: u32,
    bar_width: i32,
}

#[derive(Debug)]
struct WindowItem {
    id: usize,
    splits_from: Vec<usize>,
    splits_into: Vec<usize>,
    window: Window,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    reverts: Vec<GeomRevert>,
    split_depth: i32,
}

#[derive(Debug)]
struct GeomRevert {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

struct Key<'a> {
    key: u8,
    modf: Option<x::KeyButMask>,
    func: fn(&mut State<'a>, &[&str]) -> xcb::Result<()>,
    args: &'static [&'static str],
}

fn add_window(mut state: State, w: Window) -> xcb::Result<State> {
    //let max_split_depth = 3;
    let mut win_item: WindowItem;

    // default full screen if no other windows open
    win_item = WindowItem {
        id: state.item_list.len(),
        splits_from: Vec::<usize>::new(),
        splits_into: Vec::<usize>::new(),
        window: w,
        x: 0,
        y: state.bar_width,
        width: state.scr.width_in_pixels() as u32,
        height: state.scr.height_in_pixels() as u32 - state.bar_width as u32,
        reverts: Vec::<GeomRevert>::new(),
        split_depth: 0,
    };

    // if other windows open, modify sizes based on window new window will be split from
    if !state.item_list.is_empty() && !state.curr_win.is_empty() {
        let parent = state.item_list
            .iter()
            .position(|x| x.window == state.curr_win[0] )
            .unwrap();

        win_item.splits_from.push(state.item_list[parent].id);
        state.item_list[parent].splits_into.push(win_item.id);

        // increment split depth count
        win_item.split_depth = state.item_list[parent].split_depth + 1;
        state.item_list[parent].split_depth += 1;

        if state.item_list[parent].width > state.item_list[parent].height {
            // vertical split
            win_item.x = state.item_list[parent].x +
                (state.item_list[parent].width as i32 / 2);
            win_item.y = state.item_list[parent].y;
            win_item.width = state.item_list[parent].width / 2;
            win_item.height = state.item_list[parent].height;

            state.item_list[parent].width = win_item.width;
        } else {
            // horizontal split
            win_item.x = state.item_list[parent].x;
            win_item.y = state.item_list[parent].y +
                state.item_list[parent].height as i32 / 2;
            win_item.width = state.item_list[parent].width;
            win_item.height = state.item_list[parent].height / 2;

            state.item_list[parent].height = win_item.height;
        }
    }

    // kept for the option of window split limits
    /*if win_item.split_depth <= max_split_depth {
        state.item_list.push(win_item);
    }*/

    state.item_list.push(win_item);

    // draw windows
    for i in &state.item_list {
        let cookie = state.con.send_request_checked(&x::ConfigureWindow {
            window: i.window,
            value_list: &[
                x::ConfigWindow::X(i.x),
                x::ConfigWindow::Y(i.y),
                x::ConfigWindow::Width(i.width-state.border-2),
                x::ConfigWindow::Height(i.height-state.border-2),
                x::ConfigWindow::BorderWidth(state.border),
            ],
        });

        state.con.check_request(cookie)?;

        let cookie = state.con.send_request_checked(&x::ChangeWindowAttributes {
            window: i.window,
            value_list: &[
                x::Cw::BorderPixel(0x444444),
                x::Cw::EventMask(EventMask::KEY_PRESS |
                                 EventMask::ENTER_WINDOW |
                                 EventMask::LEAVE_WINDOW |
                                 EventMask::STRUCTURE_NOTIFY),
            ]
        });

        state.con.check_request(cookie)?;

        let cookie = state.con.send_request_checked(&x::GrabKey {
            owner_events: true,
            grab_window: i.window,
            modifiers: x::ModMask::N1 | x::ModMask::SHIFT,
            key: x::GRAB_ANY,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        });

        state.con.check_request(cookie)?;

        state.con.send_request(&x::MapWindow {
            window: i.window,
        });
    }

    state.con.flush()?;

    Ok(state)
}

fn focus(opt: bool, con: &xcb::Connection,  win: Window) -> xcb::Result<()> {
    match opt {
        // focus
        true => {
            let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
                window: win,
                value_list: &[
                    x::Cw::BorderPixel(0x0099dd),
                ],
            });

            con.check_request(cookie)?;

            let cookie = con.send_request_checked(&x::SetInputFocus {
                revert_to: x::InputFocus::PointerRoot,
                focus: win,
                time: x::CURRENT_TIME,
            });

            con.check_request(cookie)?;

            let cookie = con.send_request_checked(&x::ConfigureWindow {
                window: win,
                value_list: &[
                    x::ConfigWindow::StackMode(x::StackMode::Above),
                ]
            });

            con.check_request(cookie)?;
        }
        // defocus
        false => {
            let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
                window: win,
                value_list: &[
                    x::Cw::BorderPixel(0x444444),
                ],
            });

            con.check_request(cookie)?;
        }
    }

    Ok(())
}

impl<'a> State<'a> {
    fn destroy_win(&mut self, _args: &[&str]) -> xcb::Result<()> {
        let this_window = self.item_list
            .iter()
            .position(|x| x.window == self.curr_win[0])
            .unwrap();

        let cookie = self.con.send_request_checked(&x::DestroyWindow {
            window: self.curr_win[0],
        });

        self.con.check_request(cookie)?;

        self.item_list.remove(this_window);

        Ok(())
    }

    // nudge pushes smaller windows to edges of screen,
    // uses vim bindings hjkl for what direction in which to nudge
    fn nudge(&mut self, opt: &[&str]) -> xcb::Result<()> {
        if !self.curr_win.is_empty() {
            // get window's current dimensions
            let cookie = self.con.send_request(&x::GetGeometry {
                drawable: x::Drawable::Window(self.curr_win[0]),
            });
            let reply = self.con.wait_for_reply(cookie)?;

            // get index of current window in item_list
            let index = self.item_list
                .iter()
                .position(|x| x.window == self.curr_win[0])
                .unwrap();

            // init with garbage value because it will be overwritten
            let mut vals: Box<[x::ConfigWindow]> = Box::new([]);
            let scr_width = self.scr.width_in_pixels() as u32;
            let scr_height = self.scr.height_in_pixels() as u32;

            match opt[0] {
                "up" => {
                    if reply.y() == self.item_list[index].y as i16 {
                        self.item_list[index].y = self.bar_width;
                        self.item_list[index].height = scr_height;
                        vals = Box::new([
                            x::ConfigWindow::Y(self.bar_width),
                            x::ConfigWindow::Height(
                                scr_height - self.bar_width as u32 - self.border*2
                            ),
                        ]);
                    }
                }
                "left" => {
                    if reply.x() == self.item_list[index].x as i16 {
                        self.item_list[index].x = 0;
                        self.item_list[index].width = scr_width;
                        vals = Box::new([
                            x::ConfigWindow::X(0),
                            x::ConfigWindow::Width(scr_width - self.border*2),
                        ]);
                    }
                }
                "down" => {
                    println!("{:?}, {:?}", reply.height(), self.item_list[index].height);
                    if reply.height() ==
                        (self.item_list[index].height - (self.border*2)) as u16 {

                        self.item_list[index].height =
                            scr_height - reply.y() as u32;
                        println!("height {:?}", self.item_list[index].height);
                        vals = Box::new([
                            x::ConfigWindow::Y(self.item_list[index].y),
                            x::ConfigWindow::Height(self.item_list[index].height),
                        ]);
                    }
                }
                "right" => {
                    if reply.width() ==
                        (self.item_list[index].width - (self.border*2)) as u16 {

                        self.item_list[index].width =
                            (scr_width - reply.x() as u32) -
                                self.border*2;
                        vals = Box::new([
                            x::ConfigWindow::X(self.item_list[index].x),
                            x::ConfigWindow::Width(self.item_list[index].width),
                        ]);
                    }
                }
                "reset" => {
                    let revert = self.item_list[index].reverts.pop();
                    let item = &mut self.item_list[index];

                    match revert {
                        Some(r) => {
                            item.x = r.x;
                            item.y = r.y;
                            item.width = r.width;
                            item.height = r.height;

                            vals = Box::new([
                                x::ConfigWindow::X(r.x),
                                x::ConfigWindow::Y(r.y),
                                x::ConfigWindow::Width(r.width),
                                x::ConfigWindow::Height(r.height),
                            ]);
                        }
                        None => { println!("no more possible reversions for that window") }
                    }
                }
                _ => {}
            }

            if opt[0] != "reset" {
                self.item_list[index].reverts.push(GeomRevert {
                    x: reply.x() as i32,
                    y: reply.y() as i32,
                    width: reply.width() as u32,
                    height: reply.height() as u32,
                });
            }

            let cookie = self.con.send_request_checked(&x::ConfigureWindow {
                window: self.curr_win[0],
                value_list: &vals,
            });

            self.con.check_request(cookie)?;
        }

        Ok(())
    }

    // spawn new system process
    fn spawn(&mut self, in_args: &[&str]) -> xcb::Result<()> {
        let (command, args) = in_args.split_at(1);

        Command::new(command[0])
            .args(args)
            .spawn()
            .expect("failed to spawn");

        Ok(())
    }
}

fn main() -> xcb::Result<()> {
    let (connection, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = connection.get_setup();
    let screen = setup.roots().nth(scr_num as usize).unwrap();

    let mut state = State {
        con: &connection,
        scr: &screen,
        curr_win: Vec::<Window>::new(),
        item_list: Vec::<WindowItem>::new(),
        border: 2,
        bar_width: 13,
    };

    // --------
    // KEYBINDS
    // --------
    let keys = vec![
        Key{key: 24, modf:Some(KeyButMask::MOD1 | KeyButMask::SHIFT),
            func: State::destroy_win, args: &[""]},
        Key{key: 36, modf:Some(KeyButMask::MOD1), func: State::spawn,
            args: &["zsh", "-c", "st"]},
        Key{key: 40, modf:Some(KeyButMask::MOD1), func: State::spawn,
            args: &["zsh", "-c", "dmenu_run"]},
        Key{key: 45, modf:Some(KeyButMask::MOD1), func: State::nudge, args: &["up"]},
        Key{key: 43, modf:Some(KeyButMask::MOD1), func: State::nudge, args: &["left"]},
        Key{key: 44, modf:Some(KeyButMask::MOD1), func: State::nudge, args: &["down"]},
        Key{key: 46, modf:Some(KeyButMask::MOD1), func: State::nudge, args: &["right"]},
        Key{key: 27, modf:Some(KeyButMask::MOD1), func: State::nudge, args: &["reset"]},
    ];

    // set root attributes
    let cookie = state.con.send_request_checked(&x::ChangeWindowAttributes {
        window: state.scr.root(),
        value_list: &[
            x::Cw::BackPixel(state.scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::SUBSTRUCTURE_REDIRECT),
        ]
    });

    state.con.check_request(cookie)?;

    // set wm name
    state.con.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: state.scr.root(),
        property: x::ATOM_WM_NAME,
        r#type: x::ATOM_STRING,
        data: b"microsof-twindows",
    });

    // main loop
    loop {
        match state.con.wait_for_event()? {
            // keypress
            xcb::Event::X(x::Event::KeyPress(e)) => {

                if e.detail() == 26 &&
                    e.state() == x::KeyButMask::MOD1 | x::KeyButMask::SHIFT {

                    break Ok(());
                }

                for i in &keys {
                    if i.key == e.detail() && i.modf.unwrap() == e.state() {
                        (i.func)(&mut state, i.args)?;
                    }
                }
            }

            // unmap
            xcb::Event::X(x::Event::UnmapNotify(_e)) => {
                let remove_win = state.item_list
                    .iter()
                    .position(|x| x.window == _e.event());
                match remove_win {
                    Some(win) => {
                        state.item_list.remove(win);
                    }
                    None => {}
                }
            }

            // enter
            xcb::Event::X(x::Event::EnterNotify(_e)) => {
                state.curr_win.push(_e.event());
                focus(true, &state.con, _e.event())?;
            }

            // leave
            xcb::Event::X(x::Event::LeaveNotify(_e)) => {
                state.curr_win.pop();

                // if win_list contains _e's Window
                if state.item_list.iter().any(|x| x.window == _e.event()) {
                    focus(false, &state.con, _e.event())?;
                }
            }

            // map
            xcb::Event::X(x::Event::MapRequest(_e)) => {
                state = add_window(state, _e.window())?;
            }

            _ => {}
        }
    }
}
