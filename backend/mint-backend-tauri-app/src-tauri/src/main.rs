// src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod connection;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyTuple};
use std::fs;
use std::env;

use serde_json::json;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use rand::Rng;
use chrono::{Utc, TimeZone, DateTime};
use dotenv::dotenv;
use db::{
    initialize_connection, 
    initialize_db, 
    add_user, 
    get_users, 
    DbClient,
    add_testtime_series_data,
    get_testtime_series_data,
};

#[tauri::command]
async fn add_user_command(state: tauri::State<'_, DbClient>, name: String, email: String) -> Result<String, String> {
    add_user(state.inner().clone(), name, email)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn initialize_db_command(state: tauri::State<'_, DbClient>) -> Result<String, String> {
    initialize_db(state.inner().clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_users_command(state: tauri::State<'_, DbClient>) -> Result<Vec<(i32, String, String)>, String> {
    get_users(state.inner().clone())
        .await
        .map_err(|e| e.to_string())
}

/// Expects a timestamp in milliseconds, converts it to DateTime<Utc>, and inserts the data.
#[tauri::command]
async fn add_testtime_series_data_command(
    state: tauri::State<'_, DbClient>, 
    timestamp: i64, 
    value: f64, 
    metadata: String
) -> Result<String, String> {
    let dt: DateTime<Utc> = Utc
        .timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
        .single()
        .ok_or("Invalid timestamp")?;
    add_testtime_series_data(state.inner().clone(), dt, value, metadata)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves the time series data and returns DateTime<Utc> as an RFC3339 string.
#[tauri::command]
async fn get_testtime_series_data_command(state: tauri::State<'_, DbClient>) -> Result<Vec<(i32, String, f64, String)>, String> {
    let data = get_testtime_series_data(state.inner().clone())
        .await
        .map_err(|e| e.to_string())?;
    Ok(data.into_iter().map(|(id, dt, value, meta)| (id, dt.to_rfc3339(), value, meta)).collect())
}

#[tauri::command]
fn greet(name: &str) -> String {
    println!("inside rust code");
    format!("hello {}!", name)
}

#[tauri::command]
fn run_python_script() {
    println!("inside rust code");
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {:?}", current_dir);

    Python::with_gil(|py| {
        let script = fs::read_to_string("scripts/hello.py")
            .expect("Failed to read Python script");

        let module = PyModule::from_code(py, &script, "hello.py", "hello")
            .expect("Failed to create Python module");

        let greet_func = module.getattr("test")
            .expect("Failed to get 'test' function")
            .to_object(py);

        let args = PyTuple::new(py, &[20, 30]);
        let result = greet_func.call1(py, args)
            .expect("Failed to call 'test' function");

        let result_str: String = result.extract(py)
            .expect("Failed to extract result as String");

        println!("Result from Python: {}", result_str);
    });
}

#[tauri::command]
async fn select_model(filter: String, model_type: String) -> Result<String, String> {
    Ok(format!("Selected model with filter '{}' and type '{}'", filter, model_type))
}

fn simulate_eeg_data() -> String {
    let mut rng = rand::thread_rng();
    let data = vec![
        json!({"timestamp": Utc::now().timestamp_millis(), "channel": "1", "value": rng.gen_range(10.0..20.0)}),
        json!({"timestamp": Utc::now().timestamp_millis(), "channel": "2", "value": rng.gen_range(10.0..20.0)}),
        json!({"timestamp": Utc::now().timestamp_millis(), "channel": "3", "value": rng.gen_range(10.0..20.0)}),
    ];
    serde_json::to_string(&data).unwrap()
}

fn main() {
    dotenv().ok();
    let db_client = tauri::async_runtime::block_on(async {
        // initialize_connection().await.expect("Failed to initialize db")
    });
    tauri::Builder::default()
        .manage(db_client)
        .invoke_handler(tauri::generate_handler![
            greet, 
            run_python_script,
            select_model,
            add_user_command, 
            initialize_db_command,
            get_users_command,
            add_testtime_series_data_command,
            get_testtime_series_data_command,
        ])
        .setup(|_app| {
             tauri::async_runtime::spawn(async {connection::run_server().await;});
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
