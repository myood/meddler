#[macro_use]
extern crate windows_service;

use std::ffi::OsString;
use windows_service::{
    Result,
    service_dispatcher, 
    service::{ServiceControl, ServiceStatus, ServiceState, ServiceType, ServiceControlAccept, ServiceExitCode},
    service_control_handler::{ServiceStatusHandle, ServiceControlHandlerResult}
};

use std::sync::Mutex;
use willhook::hook::Hook;
static PROTECTED_WILLHOOK: Mutex<Option<Hook>> = Mutex::new(None);
static PROTECTED_SERVICE_HANDLE: Mutex<Option<ServiceStatusHandle>> = Mutex::new(None);

define_windows_service!(ffi_service_main, meddler_service);
#[derive(Clone, Copy)]
enum MeddlerErrorCodes {
    WillhookHandleInvalid = 0x1,
    WillhookHandleAccessError = 0x2,
    ServiceHandleInvalid = 0x3,
}

fn get_willhook_status() -> bool {
    if let Ok(willhook_guard) = PROTECTED_WILLHOOK.lock() {
        if let Some(_) = *willhook_guard {
            return true
        }
    }
    
    false
}

fn get_service_handle_status() -> bool {
    if let Ok(willhook_guard) = PROTECTED_WILLHOOK.lock() {
        if let Some(_) = *willhook_guard {
            return true
        }
    }
    
    false
}

fn get_service_status() -> ServiceControlHandlerResult {
    let willhook_status = get_willhook_status();
    if !willhook_status
    {
        return ServiceControlHandlerResult::Other(MeddlerErrorCodes::WillhookHandleInvalid as u32)
    }

    let service_handle_status = get_service_handle_status();
    if !service_handle_status
    {
        return ServiceControlHandlerResult::Other(MeddlerErrorCodes::ServiceHandleInvalid as u32)
    }

    ServiceControlHandlerResult::NoError
}

fn stop_service() -> ServiceControlHandlerResult {
    if let Ok(mut willhook_guard) = PROTECTED_WILLHOOK.lock() {
        *willhook_guard = None;
        return ServiceControlHandlerResult::NoError
    }
    
    ServiceControlHandlerResult::Other(MeddlerErrorCodes::WillhookHandleAccessError as u32)
}

fn service_control(service_control: ServiceControl) -> ServiceControlHandlerResult {
    match service_control {
        ServiceControl::Interrogate => get_service_status(),
        ServiceControl::Stop => stop_service(),
        _ => ServiceControlHandlerResult::NotImplemented,
    }
}

fn report_service_state(handle: &ServiceStatusHandle, state: ServiceState) -> Result<()> {
    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: state,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: std::time::Duration::default(),
        // Unused for setting status
        process_id: None,
    };

    // Tell the system that the service is running now
    handle.set_service_status(next_status)
}


fn meddler_service(_arguments: Vec<OsString>) {
    if let Ok(status_handle) = windows_service::service_control_handler::register("Meddler", service_control) {
        if let Ok(mut service_handle) = PROTECTED_SERVICE_HANDLE.lock() {
            *service_handle = Some(status_handle)
        }

        if let Ok(mut willhook_guard) = PROTECTED_WILLHOOK.lock() {
            *willhook_guard = willhook::willhook();
            match *willhook_guard {
                Some(_) => report_service_state(&status_handle, ServiceState::Running).unwrap(),
                _ => report_service_state(&status_handle, ServiceState::Stopped).unwrap()
            }
        }
    }
}

fn main() -> Result<()> {
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start("myservice", ffi_service_main)?;
    Ok(())
}