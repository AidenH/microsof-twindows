use xcb::{x::{self, Screen}, x::EventMask, Connection};

fn create_win(con: &Connection, scr: &Screen, root: &x::Window) -> xcb::Result<()> {
    let root_win = root.clone();
    let win: x::Window = con.generate_id();

    let cookie = con.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: win,
        parent: root_win,
        x: 60,
        y: 10,
        width: 10,
        height: 10,
        border_width: 1,
        class: x::WindowClass::InputOutput,
        visual: scr.root_visual(),
        value_list: &[
            x::Cw::BackPixel(scr.white_pixel()),
            x::Cw::EventMask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY | EventMask::STRUCTURE_NOTIFY),
        ],
    });

    con.check_request(cookie)?;

    con.send_request(&x::MapWindow {
        window: win,
    });

    con.flush()?;

    Ok(())
}

fn main() -> xcb::Result<()> {
    let (con, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = con.get_setup();
    let scr = setup.roots().nth(scr_num as usize).unwrap();

    // ROOT WINDOW
    let root_win: x::Window = con.generate_id();

    let cookie = con.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: root_win,
        parent: scr.root(),
        x: 60,
        y: 10,
        width: 300,
        height: 300,
        border_width: 1,
        class: x::WindowClass::InputOutput,
        visual: scr.root_visual(),
        value_list: &[
            x::Cw::BackPixel(scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_NOTIFY | EventMask:: SUBSTRUCTURE_REDIRECT),
        ],
    });

    con.check_request(cookie)?;

    con.send_request(&x::MapWindow {
        window: root_win,
    });

    con.flush()?;

    loop {
        match con.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(ev)) => {
                if ev.detail() == 58 { // 'm'
                    break Ok(());
                } else if ev.detail() == 38 { // 'a'
                    create_win(&con, &scr, &root_win)?;
                }
            }

            xcb::Event::X(x::Event::Expose(_)) => {
                //println!("{:?}", ev);
            }

            xcb::Event::X(x::Event::ConfigureNotify(_)) => {
                println!("config");
            }

            xcb::Event::X(x::Event::MapNotify(e)) => {
                println!("map: {:?}", e.window());
                let win = e.window();

                if win != root_win {
                    con.send_request(&x::ConfigureWindow {
                        window: win,
                        value_list: &[
                            x::ConfigWindow::X(30),
                            x::ConfigWindow::Y(30),
                            x::ConfigWindow::Width(100),
                            x::ConfigWindow::Height(100),
                        ],
                    });
                    con.flush()?;
                }
            }

            _ => {}
        }
    }
}
