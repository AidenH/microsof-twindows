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
        ],
    });

    con.check_request(cookie)?;

    let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
        window: w,
        value_list: &[
            x::Cw::EventMask(EventMask::ENTER_WINDOW | EventMask::LEAVE_WINDOW),
        ]
    });

    con.check_request(cookie)?;

    con.send_request(&x::MapWindow {
        window: w,
    });

    con.flush()?;

    Ok(())
}

/*fn destroy_win(win_list: Vec<Window>, e: Window) {
    let a = win_list.iter().position(|&x| x == e;
}*/

fn main() -> xcb::Result<()> {
    let mut win_list = Vec::<Window>::new();

    let (con, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = con.get_setup();
    let scr = setup.roots().nth(scr_num as usize).unwrap();

    let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
        window: scr.root(),
        value_list: &[
            x::Cw::BackPixel(scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::STRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT),
        ]
    });

    con.check_request(cookie)?;

    loop {
        match con.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(e)) => {
                //println!("{:?}", e);
                //println!("WINDOWS: {:?}", win_list);

                if e.detail() == 58 { // 'm'
                    break Ok(());
                } else if e.detail() == 38 { // 'a'
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/feh /home/lurkcs/Pictures/fin.jpg")
                        .spawn()
                        .expect("unable to launch");
                } else if e.detail() == 36 && e.state() == x::KeyButMask::MOD1 { // alt ent
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/st")
                        .spawn()
                        .expect("no st");
                } else if e.detail() == 40 && e.state() == x::KeyButMask::MOD1 { // alt d
                    Command::new("zsh")
                        .arg("-c")
                        .arg("/usr/bin/dmenu_run")
                        .spawn()
                        .expect("unable to load qutebrowser");
                } else if e.detail() == 24 && e.state() == x::KeyButMask::MOD1 {
                    //destroy_win(win_list, )
                }
            }

            /*xcb::Event::X(x::Event::CreateNotify(e)) => {
                println!("{:?}", e);
                //if e.override_redirect() == false {
                add_window(&con, e.window())?;
                win_list.push(e.window());
                //}
            }*/

            xcb::Event::X(x::Event::EnterNotify(_e)) => {
                println!("enter");
            }

            xcb::Event::X(x::Event::LeaveNotify(_e)) => {
                println!("leave");
            }

            xcb::Event::X(x::Event::MapRequest(_e)) => {
                //println!("{:?}", _e);
                add_window(&con, _e.window())?;
                win_list.push(_e.window());
                //println!("{:?}", win_list);
            }

            _ => {}
        }
    }
}
