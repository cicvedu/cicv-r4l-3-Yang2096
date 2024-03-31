// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use core::ops::Deref;
use core::result::Result::Err;

use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, file};

const GLOBALMEM_SIZE: usize = 0x1000;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

static GLOBALMEM_BUF: Mutex<[u8;GLOBALMEM_SIZE]> = unsafe {
    Mutex::new([0u8;GLOBALMEM_SIZE])
};

static BUF_TAIL: Mutex<usize> = unsafe {
    Mutex::new(0)
};

static BUF_HEAD: Mutex<usize> = unsafe {
    Mutex::new(0)
};

struct RustFile {
    #[allow(dead_code)]
    inner: &'static Mutex<[u8;GLOBALMEM_SIZE]>,
    tail: &'static Mutex<usize>,
    head: &'static Mutex<usize>,
}

#[vtable]
impl file::Operations for RustFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        Ok(
            Box::try_new(RustFile {
                inner: &GLOBALMEM_BUF,
                tail: &BUF_TAIL,
                head: &BUF_HEAD,
            })?
        )
    }

    fn write(_this: &Self,_file: &file::File,_reader: &mut impl kernel::io_buffer::IoBufferReader,_offset:u64,) -> Result<usize> {
        let data = _reader.read_all()?;
        pr_info!("got write request, data input {}", data.len());
        let mut inner = _this.inner.lock();
        let mut tail = _this.tail.lock();

        let mut len = data.len();
        if *tail + len >= GLOBALMEM_SIZE {
            len = GLOBALMEM_SIZE - *tail;
        }
        for i in 0..len {
            inner[i+*tail] = data[i];
        }
        *tail += len;
        Ok(data.len())
    }

    fn read(_this: &Self,_file: &file::File,_writer: &mut impl kernel::io_buffer::IoBufferWriter,_offset:u64,) -> Result<usize> {
        let inner = _this.inner.lock();
        let mut tail = _this.tail.lock();
        let mut head = _this.head.lock();
        
        let len = *tail - *head;
        pr_info!("got read request, data remaining {}", len);
        if len > 0 {
            _writer.write_slice(&inner.deref()[*head..*tail])?;
            *head = *tail;
        } else {
            *tail = 0;
            *head = 0;
        }
        Ok(len)
    }
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for RustChrdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        // Register the same kind of device twice, we're just demonstrating
        // that you can use multiple minors. There are two minors in this case
        // because its type is `chrdev::Registration<2>`
        chrdev_reg.as_mut().register::<RustFile>()?;
        chrdev_reg.as_mut().register::<RustFile>()?;

        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
