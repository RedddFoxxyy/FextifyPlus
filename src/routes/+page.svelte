<script>
  // @ts-nocheck
  import { onMount } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import '../styles/styles.css';
  import { v4 as uuidv4 } from 'uuid';
  import EditorJS from '@editorjs/editorjs';
  import Header from '@editorjs/header';
  import List from '@editorjs/list';
  import CodeTool from '@editorjs/code';
  import Quote from '@editorjs/quote';
  import Marker from '@editorjs/marker';
  import InlineCode from '@editorjs/inline-code';

  // State management
  let titleText = $state('');
  let textboxText = '';
  let recentDocuments = [];
  let currentId = $state('');
  let wordCount = $state(0);
  let charCount = $state(0);
  let tabs = $state([]);

  // Initialize Editor.js
  let editor;
  const EDITOR_JS_TOOLS = {
    header: {
      class: Header,
      inlineToolbar: true
    },
    list: {
      class: List,
      inlineToolbar: true
    },
    code: CodeTool,
    quote: Quote,
    marker: Marker,
    inlineCode: InlineCode
  };

  onMount(() => {
    // Initialize Editor.js
    editor = new EditorJS({
      holder: 'editor',
      tools: EDITOR_JS_TOOLS,
      placeholder: 'start typing...',
      onChange: async () => {
        const data = await editor.save();
        const text = data.blocks
          .map(block => block.data.text || '')
          .join(' ');
        wordCount = countWords(text);
        charCount = text.length;
      },
    });

    loadRecentDocuments();
    
    // If no documents were loaded, create a new tab
    if (recentDocuments.length === 0) {
      addnewtab();
    }

    // Set up auto-save
    const autoSaveInterval = setInterval(autoSave, 1000);

    return () => {
      clearInterval(autoSaveInterval);
      if (editor) {
        editor.destroy();
      }
    };
  });

  function countWords(text) {
    return text.split(/\s+/).filter(word => word.length > 0).length;
  }

  async function addnewtab() {
    const newTab = await invoke('new_tab');
    tabs = [newTab];
    currentId = newTab.id;
  }

  async function switchTab(tabId) {
    try {
      const docResult = await invoke('get_document_content', { id: tabId });
      
      if (docResult) {
        currentId = tabId;
        titleText = docResult.title;
        if (editor) {
          editor.render(JSON.parse(docResult.content));
        }
      } else {
        currentId = tabId;
        titleText = '';
        if (editor) {
          editor.clear();
        }
      }
    } catch (error) {
      console.error('Failed to switch tab:', error);
    }
  }

  async function autoSave() {
    if (!titleText && !editor) return;

    try {
      const savedData = await editor.save();
      await invoke('save_document', {
        id: currentId,
        title: titleText,
        content: JSON.stringify(savedData)
      });
    } catch (error) {
      console.error('Auto-save failed:', error);
    }
  }

  async function loadRecentDocuments() {
    try {
      const docs = await invoke('load_recent_files');
      recentDocuments = docs;
      
      if (recentDocuments.length > 0) {
        tabs = recentDocuments.map(doc => ({
          order: tabs.length + 1,
          id: doc.id,
          title: doc.title
        }));
        
        const lastDoc = recentDocuments[recentDocuments.length - 1];
        currentId = lastDoc.id;
        titleText = lastDoc.title;
        if (editor) {
          editor.render(JSON.parse(lastDoc.content));
        }
      }
    } catch (error) {
      console.error('Failed to load documents:', error);
    }
  }

  function handleTitleChange(event) {
    titleText = event.target.value;
  }

  function handleKeydown(event) {
    if (event.ctrlKey && event.key === 'd') {
      event.preventDefault();
      deleteDocument();
    }
    if (event.ctrlKey && event.key === 'n') {
      event.preventDefault();
      newDocument();
    }
  }

  async function deleteDocument() {
    try {
      await invoke('delete_document', { id: currentId });
      tabs = tabs.filter(tab => tab.id !== currentId);
      
      if (tabs.length > 0) {
        const lastTab = tabs[tabs.length - 1];
        currentId = lastTab.id;
        const docResult = await invoke('get_document_content', { id: currentId });
        titleText = docResult.title;
        if (editor) {
          editor.render(JSON.parse(docResult.content));
        }
      } else {
        await invoke('reset_tab_order_count')
        await newDocument();
      }
    } catch (error) {
      console.error('Failed to delete document:', error);
    }
  }

  async function newDocument() {
    try {
      const newTab = await invoke('new_tab');
      tabs = [...tabs, newTab];
      currentId = newTab.id;
      titleText = '';
      if (editor) {
        editor.clear();
      }
    } catch (error) {
      console.error('Failed to create new document:', error);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown}/>

<main class="container">
  <div class="title-container title-textarea">
    <textarea
      class="rounded-container"
      placeholder="Enter Title here..."
      value={titleText}
      oninput={handleTitleChange}
    ></textarea>
  </div>
  
  <div id="editor" class="editorjs-container"></div>
  
  <div class="word-char-counter">
    {wordCount} Words {charCount} Characters
  </div>

  <div class="tab-counter" role="tablist" aria-label="Document tabs">
    {#each tabs as tab}
      <button
        type="button"
        class="tab-square"
        class:active={currentId === tab.id}
        role="tab"
        aria-selected={currentId === tab.id}
        aria-controls="editor"
        onclick={() => switchTab(tab.id)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            switchTab(tab.id);
          }
        }}
      >
        {tab.order}
      </button>
    {/each}
  </div>
</main>