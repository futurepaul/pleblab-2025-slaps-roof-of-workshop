// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// lib.rs contains our app implementation
use slaps_roof_of_wallet_lib::run;

fn main() {
    run();
}
