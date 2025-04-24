use gtk::{Application, ApplicationWindow, Box, Label, Stack, StackSwitcher, glib, prelude::*};

const APP_ID: &str = "org.ppt_stealer-rs.StealerInterface";

/// Main launcher function exposed as API to GTK based GUI
pub fn main_launcher() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    // The ui has a `GtkNotebook`, with tabs on the top for switching
    // between different views. Tabs are:
    // - Home
    // - Source
    // - Local Destination
    // - SSH Destination
    // - SMB Destination
    // - About

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("PPT Stealer Interface")
        .default_width(800)
        .default_height(600)
        .build();

    // 创建垂直布局容器
    let main_box = Box::new(gtk::Orientation::Vertical, 0);

    // 创建 Stack 和 StackSwitcher
    let stack = Stack::new();
    let stack_switcher = StackSwitcher::new();
    stack_switcher.set_stack(Some(&stack));

    // 设置 Stack 的切换动画
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    stack.set_transition_duration(200);

    // 创建各个页面内容（placeholders）
    let home_page = Label::new(Some("Home Content"));
    let source_page = Label::new(Some("Source Settings"));
    let local_dest_page = Label::new(Some("Local Destination Settings"));
    let ssh_dest_page = Label::new(Some("SSH Destination Settings"));
    let smb_dest_page = Label::new(Some("SMB Destination Settings"));
    let about_page = Label::new(Some("About Information"));

    // 将页面添加到 Stack 并设置标题
    stack.add_titled(&home_page, Some("home"), "Home");
    stack.add_titled(&source_page, Some("source"), "Source");
    stack.add_titled(&local_dest_page, Some("local"), "Local Destination");
    stack.add_titled(&ssh_dest_page, Some("ssh"), "SSH Destination");
    stack.add_titled(&smb_dest_page, Some("smb"), "SMB Destination");
    stack.add_titled(&about_page, Some("about"), "About");

    // 将控件添加到主容器
    main_box.append(&stack_switcher);
    main_box.append(&stack);

    // 设置窗口主内容
    window.set_child(Some(&main_box));
    window.present();
}
