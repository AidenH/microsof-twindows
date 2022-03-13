use std::process::Command;

use xcb::{x::{self, Window}, x::EventMask};

struct State<'a> {
    con: &'a xcb::Connection,
    scr: &'a x::Screen,
    wins: Vec<Window>,
    curr_win: Vec<Window>,
}

fn add_window(con: &xcb::Connection, w: Window) -> xcb::Result<()> {
    let cookie = con.send_request_checked(&x::ConfigureWindow {
        window: w,
        value_list: &[
            x::ConfigWindow::X(100),
            x::ConfigWindow::Y(100),
            x::ConfigWindow::Width(400),
            x::ConfigWindow::Height(400),
            x::ConfigWindow::BorderWidth(2),
        ],
    });

    con.check_request(cookie)?;

    let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
        window: w,
        value_list: &[
            x::Cw::BorderPixel(0x444444),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::ENTER_WINDOW | EventMask::LEAVE_WINDOW | EventMask::STRUCTURE_NOTIFY),
        ]
    });

    con.check_request(cookie)?;

    con.send_request(&x::MapWindow {
        window: w,
    });

    con.flush()?;

    Ok(())
}

fn destroy_win(state: &State) -> xcb::Result<usize> {
    let cookie = state.con.send_request_checked(&x::DestroyWindow {
        window: state.curr_win[0],
    });

    state.con.check_request(cookie)?;

    let a = state.wins.iter().position(|&x| x == state.curr_win[0]).unwrap();

    Ok(a)
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

fn redraw_tiles() {
    // tiling logic
}

fn main() -> xcb::Result<()> {
    let (connection, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = connection.get_setup();
    let screen = setup.roots().nth(scr_num as usize).unwrap();

    let mut state = State {
        con: &connection,
        scr: &screen,
        wins: Vec::<Window>::new(),
        curr_win: Vec::<Window>::new(),
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
                        .expect("no st");
                } else if e.detail() == 40 && e.state() == x::KeyButMask::MOD1 { // alt 'd'
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/dmenu_run")
                        .spawn()
                        .expect("unable to load qutebrowser");
                } else if e.detail() == 24 &&
                    e.state() == x::KeyButMask::MOD1 | x::KeyButMask::SHIFT { // alt 'q'

                    if !state.curr_win.is_empty() && !state.wins.is_empty() {
                        let remove_win = destroy_win(&state).unwrap();
                        state.wins.remove(remove_win);
                    }
                }
            }

            xcb::Event::X(x::Event::UnmapNotify(_e)) => {
                let remove_win = state.wins.iter().position(|&x| x == _e.event());
                match remove_win {
                    Some(win) => {
                        state.wins.remove(win);
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
                if state.wins.iter().any(|&x| x == _e.event()) {
                    focus(false, &state.con, _e.event())?;
                }
            }

            xcb::Event::X(x::Event::MapRequest(_e)) => {
                add_window(&state.con, _e.window())?;
                state.wins.push(_e.window());
            }

            _ => {}
        }
    }
}
