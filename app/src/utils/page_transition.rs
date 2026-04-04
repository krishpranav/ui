pub const PAGE_OUTLET: &str = "page__outlet";

pub fn retrigger_page_fade() {
    if let Some(window) = web_sys::window()
        && let Some(document) = window.document()
        && let Some(el) = document.get_element_by_id(PAGE_OUTLET)
    {
        let cl = el.class_list();
        cl.remove_1("page__fade").ok();
        let _ = el.get_bounding_client_rect();
        cl.add_1("page__fade").ok();
    }
}
