use adw;
use gtk;
use gtk::{glib, prelude::*};

const APP_ID: &str = "org.ppt_stealer-rs.StealerInterface";

/// Main launcher function exposed as API to GTK based GUI
pub fn main_launcher() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &adw::Application) {
    // The ui has a `GtkNotebook`, with tabs on the top for switching
    // between different views. Tabs are:
    // - Home
    // - Source
    // - Local Destination
    // - SSH Destination
    // - SMB Destination
    // - About

    // Create a window
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("PPT Stealer Interface")
        .default_width(800)
        .default_height(600)
        .build();

    // 创建 HeaderBar 作为自定义标题栏
    let header_bar = gtk::HeaderBar::new();
    header_bar.set_show_title_buttons(true); // 显示窗口控制按钮

    // 创建 Stack 和 StackSwitcher
    // let stack = gtk::Stack::new();
    // let stack_switcher = gtk::StackSwitcher::new();

    // stack_switcher.set_stack(Some(&stack));

    let view_stack = adw::ViewStack::new();
    let view_switcher = adw::ViewSwitcher::new();

    view_switcher.set_stack(Some(&view_stack));

    // 将 StackSwitcher 居中放置在 HeaderBar 中
    header_bar.set_title_widget(Some(&view_switcher));

    // 配置 Stack 切换动画
    // stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    // stack.set_transition_duration(200);
    // view_stack.set_property("transition_type", gtk::StackTransitionType::SlideLeftRight);

    // 创建各个页面内容（placeholders）
    let home_page = gtk::Label::new(Some("Home Content"));
    let source_page = gtk::Label::new(Some("Source Settings"));
    let local_dest_page = gtk::Label::new(Some("Local Destination Settings"));
    let ssh_dest_page = gtk::Label::new(Some("SSH Destination Settings"));
    let smb_dest_page = gtk::Label::new(Some("SMB Destination Settings"));
    let ftp_dest_page = gtk::Label::new(Some("FTP Destination Settings"));
    let about_page = gtk::Label::new(Some("About Information"));

    // 将页面添加到 Stack 并设置标题
    view_stack
        .add_titled(&home_page, Some("home"), "Home")
        .set_icon_name(Some("home-symbolic"));
    view_stack
        .add_titled(&source_page, Some("source"), "Source")
        .set_icon_name(Some("folder-open-symbolic"));
    view_stack
        .add_titled(&local_dest_page, Some("local"), "Local Destination")
        .set_icon_name(Some("folder-download-symbolic"));
    view_stack
        .add_titled(&ssh_dest_page, Some("ssh"), "SSH Destination")
        .set_icon_name(Some("network-server-symbolic"));
    view_stack
        .add_titled(&smb_dest_page, Some("smb"), "SMB Destination")
        .set_icon_name(Some("folder-remote-symbolic"));
    view_stack
        .add_titled(&ftp_dest_page, Some("ftp"), "FTP Destination")
        .set_icon_name(Some("folder-remote-symbolic"));
    view_stack
        .add_titled(&about_page, Some("about"), "About")
        .set_icon_name(Some("help-about-symbolic"));

    // 设置窗口标题栏
    window.set_titlebar(Some(&header_bar));

    // 设置窗口主内容
    window.set_child(Some(&view_stack));
    window.present();
}
