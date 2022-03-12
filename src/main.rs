use std::process::Command;

use xcb::{x::{self, Window}, x::EventMask};

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

fn destroy_win(con: &xcb::Connection, win_list: &Vec<Window>, e: Window) -> xcb::Result<usize> {
    let cookie = con.send_request_checked(&x::DestroyWindow {
        window: e,
    });

    con.check_request(cookie)?;

    let a = win_list.iter().position(|&x| x == e).unwrap();

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

fn refresh_tiles() {
    // tiling logic
}

fn main() -> xcb::Result<()> {
    let mut win_list = Vec::<Window>::new(); // list of all open windows
    let mut curr_win = Vec::<Window>::new(); // currently focused window

    let (con, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = con.get_setup();
    let scr = setup.roots().nth(scr_num as usize).unwrap();

    let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
        window: scr.root(),
        value_list: &[
            x::Cw::BackPixel(scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::SUBSTRUCTURE_REDIRECT),
        ]
    });

    con.check_request(cookie)?;

    loop {
        match con.wait_for_event()? {
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

                    if !curr_win.is_empty() && !win_list.is_empty() {
                        let remove_win = destroy_win(&con, &win_list, curr_win[0]).unwrap();
                        win_list.remove(remove_win);
                    }
                }
            }

            xcb::Event::X(x::Event::UnmapNotify(_e)) => {
                let remove_win = win_list.iter().position(|&x| x == _e.event());
                match remove_win {
                    Some(win) => {
                        win_list.remove(win);
                    }
                    None => {}
                }
            }

            xcb::Event::X(x::Event::EnterNotify(_e)) => {
                curr_win.push(_e.event());
                focus(true, &con, _e.event())?;
            }

            xcb::Event::X(x::Event::LeaveNotify(_e)) => {
                curr_win.pop();

                // if win_list contains _e's Window
                if win_list.iter().any(|&x| x == _e.event()) {
                    focus(false, &con, _e.event())?;
                }
            }

            xcb::Event::X(x::Event::MapRequest(_e)) => {
                add_window(&con, _e.window())?;
                win_list.push(_e.window());
                refresh_tiles();
            }

            _ => {}
        }
    }
}
