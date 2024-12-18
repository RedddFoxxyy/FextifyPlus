use ev::KeyboardEvent;
// use leptos::leptos_dom::ev::SubmitEvent;
use leptos::*;
use prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlElement, Window};
use wasm_bindgen::JsCast;
use std::time::Duration;
use serde_json::json;
use uuid;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    fn initialize_quill(editor_id: &str);
    fn get_quill_content(editor_id: &str) -> String;
}

#[derive(Serialize, Deserialize, Clone)]
struct DocumentData {
    id: String,          // Unique ID for each document
    title: String,
    content: String,
}

// Function to count words, accounting for multiple whitespaces
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[component]
pub fn App() -> impl IntoView {
    let (title_text, set_title_text) = prelude::signal(String::new());
    let (textbox_text, set_textbox_text) = prelude::signal(String::new());
    let (recent_documents, set_recent_documents) = prelude::signal(Vec::<DocumentData>::new());
    let (current_id, set_current_id) = prelude::signal(String::new()); // Signal for the unique ID
    
    // New signals for word and character counts
    let (word_count, set_word_count) = prelude::signal(0);
    let (char_count, set_char_count) = prelude::signal(0);

    // Generate a new unique ID when the app starts
    let generate_id = || uuid::Uuid::new_v4().to_string();
    prelude::Effect::new(move |_| {
        if current_id.get().is_empty() {
            set_current_id.set(generate_id());
        }
    });

    Effect::new(move |_| {
        initialize_quill("editor");
    });

    Effect::new(move |_| {
        // Use spawn_local to handle async operations
        spawn_local(async move {
            let window = web_sys::window().expect("No global window exists");
            let document = window.document().expect("No document exists");
    
            // Use JsCast to convert to the right type
            let editor = document.get_element_by_id("editor")
                .expect("Editor element not found")
                .dyn_into::<HtmlElement>()
                .expect("Failed to cast to HtmlElement");
    
            // Use wasm_bindgen to inject JavaScript
            let closure = Closure::wrap(Box::new(move || {
                // Get content from Quill
                let content = get_quill_content("editor");
                
                // Update word count
                let words = count_words(&content);
                set_word_count.set(words);
                
                // Update character count 
                let chars = content.chars().count();
                set_char_count.set(chars);
    
                // Trigger auto-save
                spawn_local(async move {
                    let id = current_id.get();
                    let title = title_text.get();
            
                    // Only save if there's content
                    if !title.is_empty() || !content.is_empty() {
                        let args = serde_wasm_bindgen::to_value(&json!({
                            "id": id,
                            "title": title,
                            "content": content
                        }))
                        .unwrap();
            
                        let _ = invoke("save_document", args).await;
                    }
                });
            }) as Box<dyn Fn()>);
    
        // Add event listener using web-sys methods
        let js_function: &js_sys::Function = closure.as_ref().unchecked_ref();
        
        // Inject script to add event listener
        let script = document.create_element("script").unwrap();
        script.set_text_content(Some(&format!(r#"
            if (window.quillEditors && window.quillEditors['editor']) {{
                window.quillEditors['editor'].on('text-change', () => {{
                    ({})();
                }});
            }}
        "#, js_function)));
        
        document.body().unwrap().append_child(&script).unwrap();
    
        // Prevent the closure from being dropped
        closure.forget();
        });
    });

    // Auto-save function
    let auto_save = move || {
        spawn_local(async move {
            let id = current_id.get();
            let title = title_text.get();
            
            // Get content directly from Quill
            let content = get_quill_content("editor");
    
            // Only save if there's content
            if !title.is_empty() || !content.is_empty() {
                let args = serde_wasm_bindgen::to_value(&json!({
                    "id": id,
                    "title": title,
                    "content": content
                }))
                .unwrap();
    
                let _ = invoke("save_document", args).await;
            }
        });
    };

    // Update word and character counts whenever text changes
    Effect::new(move |_| {
        // Note: You'll need to implement a JS function in your Tauri/frontend to get Quill content
        let current_text = get_quill_content("editor");
        
        // Update word count
        let words = count_words(&current_text);
        set_word_count.set(words);
        
        // Update character count 
        let chars = current_text.chars().count();
        set_char_count.set(chars);
    });

    // Debounce auto-save (modify this to debounce less aggressively)
    Effect::new(move |_| {
        let _title = title_text.get(); // Trigger dependency
        let _content = textbox_text.get(); // Trigger dependency

        set_timeout(move || {
            auto_save();
        }, Duration::from_millis(1000));
    });

    let load_recent_documents = move || {
        spawn_local(async move {
            let json_value = invoke("load_recent_files", JsValue::NULL).await;
    
            if let Some(json_str) = json_value.as_string() {
                if let Ok(docs) = serde_json::from_str::<Vec<DocumentData>>(&json_str) {
                    if let Some(last_doc) = docs.last() {
                        set_current_id.set(last_doc.id.clone());
                        set_title_text.set(last_doc.title.clone());
                        set_textbox_text.set(last_doc.content.clone());
                    }
                    // If you still want to track recent documents
                    set_recent_documents.set(docs);
                } else {
                    //tracing::error!("Failed to parse recent documents");
                }
            } else {
                //tracing::error!("Failed to get JSON string from load_recent_files");
            }
        });
    };

    // Load documents when component mounts
    Effect::new(move |_| {
        load_recent_documents();
    });

    // Update title and textbox handlers
    let update_title = move |ev: ev::Event| {
        if let Some(input_event) = ev.dyn_into::<web_sys::InputEvent>().ok() {
            let v = event_target_value(&input_event);
            set_title_text.set(v);
        }
    };

    // let update_textbox = move |ev: ev::Event| {
    //     spawn_local(async move {
    //         let content = get_quill_content("editor");
    //         set_textbox_text.set(content);
    //     });
    // };

    // Delete document handler (Ctrl+X)
    let delete_document = move |ev: KeyboardEvent| {
        if ev.ctrl_key() && ev.key() == "x" {
            ev.prevent_default();
            let id = current_id.get_untracked();

            spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&json!({
                    "id": id
                }))
                .unwrap();

                let _ = invoke("delete_document", args).await;

                // Clear the current document
                set_current_id.set(String::new());
                set_title_text.set(String::new());
                set_textbox_text.set(String::new());
            });
        }
    };

    view! {
        <main class="container"
            on:keydown=delete_document
        >
            <div class="title-container title-textarea">
                <textarea
                    class="rounded-container"
                    placeholder="Enter Title here..."
                    prop:value=title_text
                    on:input=update_title
                />
            </div>
            // <div class="textbox-container">
            //     <textarea
            //         class="rounded-container"
            //         placeholder="Start typing..."
            //         prop:value=textbox_text
            //         on:input=update_textbox
            //     />
            // </div>
            <div id="editor" class="quillbox-container ql-container ql-editor"></div>
            <div class="word-char-counter">
                {move || format!("Words: {} | Characters: {}", word_count.get(), char_count.get())}
            </div>
        </main>
    }
}