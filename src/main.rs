use std::process::Command;

use xcb::{x::{self, Window}, x::EventMask};

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
    split_depth: i32,
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

    // only add to window rendering list if within split depth limits
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

        state.con.send_request(&x::MapWindow {
            window: i.window,
        });
    }

    state.con.flush()?;

    Ok(state)
}

fn destroy_win(mut state: State) -> xcb::Result<State> {
    let this_window = state.item_list
        .iter()
        .position(|x| x.window == state.curr_win[0])
        .unwrap();

    let cookie = state.con.send_request_checked(&x::DestroyWindow {
        window: state.curr_win[0],
    });

    state.con.check_request(cookie)?;

    // ideally for reparenting-based window swallow:
    //
    // for child in this.splits_into
        //child.parent = this.parent
        //this.child = child.parent

    state.item_list.remove(this_window);

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

fn nudge<'a>(mut state: State<'a>, opt: &str) -> xcb::Result<State<'a>> {
    let cookie = state.con.send_request(&x::GetGeometry {
        drawable: x::Drawable::Window(state.curr_win[0]),
    });

    let reply = state.con.wait_for_reply(cookie)?;

    let index = state.item_list
        .iter()
        .position(|x| x.window == state.curr_win[0])
        .unwrap();

    // init with garbage value because it will be overwritten
    let mut vals: [x::ConfigWindow; 2] =
        [x::ConfigWindow::X(0), x::ConfigWindow::X(0)];
    let scr_width = state.scr.width_in_pixels() as u32;
    let scr_height = state.scr.height_in_pixels() as u32;

    match opt {
        "up" => {
            if reply.y() == state.item_list[index].y as i16 {
                state.item_list[index].y = state.bar_width;
                state.item_list[index].height = scr_height;
                vals = [
                    x::ConfigWindow::Y(state.bar_width),
                    x::ConfigWindow::Height(
                        scr_height - state.bar_width as u32 - state.border*2
                    ),
                ];
            }
        }
        "left" => {
            if reply.x() == state.item_list[index].x as i16 {
                state.item_list[index].x = 0;
                state.item_list[index].width = scr_width;
                vals = [
                    x::ConfigWindow::X(0),
                    x::ConfigWindow::Width(scr_width - state.border*2),
                ];
            }
        }
        "down" => {
            if reply.height() == (state.item_list[index].height - (state.border*2)) as u16 {
                state.item_list[index].height =
                    (state.scr.height_in_pixels() - reply.y() as u16) as u32 -
                        state.border*2;
                vals = [
                    x::ConfigWindow::Y(state.item_list[index].y),
                    x::ConfigWindow::Height(state.item_list[index].height),
                ];
            }
        }
        "right" => {
            if reply.width() == (state.item_list[index].width - (state.border*2)) as u16 {
                state.item_list[index].width =
                    (state.scr.width_in_pixels() - reply.x() as u16) as u32 -
                        state.border*2;
                vals = [
                    x::ConfigWindow::X(state.item_list[index].x),
                    x::ConfigWindow::Width(state.item_list[index].width),
                ];
            }
        }
        _ => {}
    }

    let cookie = state.con.send_request_checked(&x::ConfigureWindow {
        window: state.curr_win[0],
        value_list: &vals,
    });

    state.con.check_request(cookie)?;

    Ok(state)
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

    let cookie = state.con.send_request_checked(&x::ChangeWindowAttributes {
        window: state.scr.root(),
        value_list: &[
            x::Cw::BackPixel(state.scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::SUBSTRUCTURE_REDIRECT),
        ]
    });

    state.con.check_request(cookie)?;

    loop {
        match state.con.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(e)) => {

                if e.detail() == 26 &&
                    e.state() == x::KeyButMask::MOD1 | x::KeyButMask::SHIFT { // ct-sh-'e'

                    break Ok(());
                } else if e.detail() == 36 && e.state() == x::KeyButMask::MOD1 { // alt ent
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/st")
                        .spawn()
                        .expect("failed to load terminal");
                } else if e.detail() == 40 && e.state() == x::KeyButMask::MOD1 { // alt 'd'
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/dmenu_run")
                        .spawn()
                        .expect("failed to load dmenu");
                } else if e.detail() == 24 &&
                    e.state() == x::KeyButMask::MOD1 | x::KeyButMask::SHIFT { // alt 'q'

                    if !state.curr_win.is_empty() && !state.item_list.is_empty() {
                        state = destroy_win(state).unwrap();
                    }
                } else if e.detail() == 111 &&
                    e.state() == x::KeyButMask::MOD1 {

                    state = nudge(state, "up")?;
                } else if e.detail() == 113 &&
                    e.state() == x::KeyButMask::MOD1 {

                    state = nudge(state, "left")?;
                } else if e.detail() == 116 &&
                    e.state() == x::KeyButMask::MOD1 {

                    state = nudge(state, "down")?;
                } else if e.detail() == 114 &&
                    e.state() == x::KeyButMask::MOD1 {

                    state = nudge(state, "right")?;
                }
            }

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

            xcb::Event::X(x::Event::EnterNotify(_e)) => {
                state.curr_win.push(_e.event());
                focus(true, &state.con, _e.event())?;
            }

            xcb::Event::X(x::Event::LeaveNotify(_e)) => {
                state.curr_win.pop();

                // if win_list contains _e's Window
                if state.item_list.iter().any(|x| x.window == _e.event()) {
                    focus(false, &state.con, _e.event())?;
                }
            }

            xcb::Event::X(x::Event::MapRequest(_e)) => {
                state = add_window(state, _e.window())?;
            }

            _ => {}
        }
    }
}
