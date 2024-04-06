// SPDX-License-Identifier: GPL-2.0

//! Rust completion module

use core::default::Default;
use core::ptr::null;
use core::result::Result::{Err, Ok};
use core::ops::Deref;

use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, bindings, file};
use kernel::task::Task;

struct CompletionNew(bindings::completion);

unsafe impl Send for CompletionNew {}


static GLOBAL_COMPLETION: Mutex<Option<CompletionNew>> = unsafe {
    Mutex::new(None)
};



struct RustCompletion {}

#[vtable]
impl file::Operations for RustCompletion {
    type Data = ();

    fn open(_context: &Self::OpenData, _file: &file::File) -> Result<Self::Data> {
        pr_info!("open is invoked\n");
        Ok(())
    }

    fn write(
            _data: <Self::Data as kernel::PointerWrapper>::Borrowed<'_>,
            _file: &file::File,
            _reader: &mut impl kernel::io_buffer::IoBufferReader,
            _offset: u64,
        ) -> Result<usize> {
        pr_info!("write is invoked\n");
        pr_info!("process {} wakening the readers...\n", Task::current().pid());

        let a : *mut bindings::completion;
        {
            let compl = GLOBAL_COMPLETION.lock();
            a = compl.deref().as_ref().map_or(null(), |p| p ) as _;
        }

        unsafe{ bindings::complete(a)};
        
        
        Ok(_reader.len())
    }

    fn read(
            _data: <Self::Data as kernel::PointerWrapper>::Borrowed<'_>,
            _file: &file::File,
            _writer: &mut impl kernel::io_buffer::IoBufferWriter,
            _offset: u64,
        ) -> Result<usize> {
        pr_info!("read is invoked\n");
        pr_info!("process {} is going to sleep\n", Task::current().pid());

        let a : *mut bindings::completion;
        {
            let compl = GLOBAL_COMPLETION.lock();
            a = compl.deref().as_ref().map_or(null(), |p| p ) as _;
        }
        unsafe{ bindings::wait_for_completion(a) };    
        pr_info!("awoken {}\n", Task::current().pid());
        Ok(0)
    }


}

module! {
    type: RustCompletionCDev,
    name: "rust_completion",
    author: "Yang2096",
    description: "Rust completion",
    license: "GPL",
}


struct RustCompletionCDev {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}


impl kernel::Module for RustCompletionCDev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        let mut compl = GLOBAL_COMPLETION.lock();
        *compl = Some(CompletionNew(bindings::completion::default()));
        let a = compl.deref().as_ref().map_or(null(), |p| p ) as *mut bindings::completion;
        unsafe {bindings::init_completion(a)};

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        chrdev_reg.as_mut().register::<RustCompletion>()?;

        Ok(RustCompletionCDev { _dev: chrdev_reg })
    }
}