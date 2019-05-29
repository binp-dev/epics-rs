use std::ops::DerefMut;

use libc::{c_int, c_void};

use epics_sys::{
    dbCommon,
    CALLBACK, callbackSetProcess,
};

use crate::{
    record::{
        Scan, Callback,
        RecordType,
    },
    util::{cstr_to_slice},
};


/// Private data in record
pub trait Private: DerefMut<Target=CommonPrivate> + Send {}

/// Common private data
pub struct CommonPrivate {
    rtype: RecordType,
    callback: Callback,
    scan: Option<Scan>,
}

/// Record that could be emerged from raw pointer
pub trait FromRaw {
    type Raw;
    unsafe fn from_raw(raw: Self::Raw) -> Self;
}

pub(crate) unsafe fn raw_private_init<P: Private>(raw: &mut dbCommon, pvt: P) {
    assert!(raw.dpvt.is_null());
    raw.dpvt = Box::leak(Box::new(pvt)) as *mut _ as *mut c_void;
}
pub(crate) unsafe fn raw_private_create(raw: &mut dbCommon, rtype: RecordType) -> CommonPrivate {
    let mut cb: CALLBACK = CALLBACK::default();
    callbackSetProcess(
        (&mut cb) as *mut _,
        raw.prio as c_int,
        raw as *mut _ as *mut c_void,
    );
    CommonPrivate {
        rtype,
        callback: Callback::new(cb),
        scan: None,
    }
}

/// Behavior that is common for all records
pub trait Record {
    unsafe fn as_raw(&self) -> &dbCommon;
    unsafe fn as_raw_mut(&mut self) -> &mut dbCommon;

    unsafe fn init(&mut self);

    fn rtype(&self) -> RecordType {
        unsafe { self.private() }.rtype
    }
    fn name(&self) -> &[u8] {
        cstr_to_slice(&unsafe { self.as_raw() }.name)
    }
    fn pact(&self) -> bool {
        unsafe { self.as_raw() }.pact != 0
    }
    unsafe fn set_pact(&mut self, pact: bool) {
        self.as_raw_mut().pact = if pact { 1 } else { 0 };
    }

    unsafe fn private(&self) -> &Private;
    unsafe fn private_mut(&mut self) -> &mut Private;

    unsafe fn process(&mut self) -> Result<(),()> {
        let pvt = self.private_mut();
        pvt.callback.request()
    }
}

/// Scannable record behavior
pub trait ScanRecord: Record {
    unsafe fn create_scan(&self) -> Scan {
        Scan::new()
    }
    unsafe fn set_scan(&mut self, scan: Scan) {
        assert!(self.private_mut().scan.replace(scan).is_none());
    }
    unsafe fn get_scan(&self) -> Option<Scan> {
        self.private().scan.clone()
    }
    unsafe fn handler_set_scan(&mut self, scan: Scan);
}

/// Readable record behavior
pub trait ReadRecord: Record {
    unsafe fn handler_read(&mut self) -> bool;
    unsafe fn handler_read_async(&mut self);
}

/// Writable record behavior
pub trait WriteRecord: Record {
    unsafe fn handler_write(&mut self) -> bool;
    unsafe fn handler_write_async(&mut self);
}