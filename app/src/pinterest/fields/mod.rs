const SET_VALUE_FN_JS: &str = r#"const setValue = (el, v) => {
    try { el.focus(); } catch(e) {}
    const tag = (el.tagName||'').toUpperCase();
    const ce = (el.getAttribute('contenteditable')||'').toLowerCase();
    if (ce === 'true' || tag === 'DIV' || tag === 'SPAN') {
      try { document.execCommand('selectAll', false, null); document.execCommand('insertText', false, v); } catch(e) {}
    } else {
      try {
        const proto = tag === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
        const ns = Object.getOwnPropertyDescriptor(proto, 'value');
        if (ns && ns.set) ns.set.call(el, v); else el.value = v;
      } catch(e) { try { el.value = v; } catch(e2) {} }
    }
    try { el.dispatchEvent(new Event('input', { bubbles: true })); } catch(e) {}
    try { el.dispatchEvent(new Event('change', { bubbles: true })); } catch(e) {}
    try { el.dispatchEvent(new Event('blur', { bubbles: true })); } catch(e) {}
  };"#;

mod fill_text_fields;
mod make_description_js;
mod make_link_js;
mod make_title_js;
mod set_all_fields;
mod truncate_description;
mod write_fields_debug;

pub(super) use fill_text_fields::fill_text_fields;
