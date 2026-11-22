"use strict";
// Browser services for the wasm code: the saved settings and board in
// localStorage (see src/settings.rs). Load after gl.js and before load().

function net_string(ptr, len) {
    return new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, ptr, len).slice());
}

// ---- settings: one localStorage entry of key=value lines
const settings_key = "rusty-sudoku-settings";
let settings_bytes = null;

function settings_len() {
    let text = null;
    try { text = localStorage.getItem(settings_key); } catch (e) { /* storage blocked */ }
    settings_bytes = text === null ? null : new TextEncoder().encode(text);
    return settings_bytes ? settings_bytes.length : -1;
}

function settings_read(dest) {
    if (settings_bytes) new Uint8Array(wasm_memory.buffer, dest, settings_bytes.length).set(settings_bytes);
    settings_bytes = null;
}

function settings_write(ptr, len) {
    try { localStorage.setItem(settings_key, net_string(ptr, len)); } catch (e) { /* storage blocked */ }
}

function net_register_plugin(importObject) {
    importObject.env.settings_len = settings_len;
    importObject.env.settings_read = settings_read;
    importObject.env.settings_write = settings_write;
}

// (named apart from audio.js's register_plugin: both are globals.)
miniquad_add_plugin({ register_plugin: net_register_plugin, version: 1, name: "rusty_settings" });
