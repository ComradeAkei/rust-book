// SPDX-License-Identifier: GPL-2.0

//! 29.7: misc-устройство /dev/ch29-notes. У каждого открытия свой буфер
//! заметок: write дописывает, read читает с позиции файла, ioctl сообщает
//! длину и очищает буфер.

use core::mem::size_of;
use kernel::{
    fs::{File, Kiocb},
    ioctl::{_IO, _IOR},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    new_mutex,
    prelude::*,
    sync::Mutex,
    uaccess::UserSlice,
};

module! {
    type: NotesModule,
    name: "ch29_notes",
    authors: ["Rust textbook"],
    description: "Per-open notes buffer as a misc device",
    license: "GPL",
    params: {
        simple: u32 {
            default: 0,
            description: "1: read through IovIterDest::simple_read_from_buffer",
        },
    },
}

const CAPACITY: usize = 4096;
// ANCHOR: ioctl
const NOTES_LEN: u32 = _IOR::<u64>(b'n' as u32, 1);
const NOTES_CLEAR: u32 = _IO(b'n' as u32, 2);
// ANCHOR_END: ioctl

#[pin_data]
struct NotesModule {
    #[pin]
    _misc: MiscDeviceRegistration<Notes>,
}

impl kernel::InPlaceModule for NotesModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            _misc <- MiscDeviceRegistration::register(MiscDeviceOptions { name: c"ch29-notes" }),
        })
    }
}

// ANCHOR: open
#[pin_data(PinnedDrop)]
struct Notes {
    #[pin]
    buf: Mutex<KVVec<u8>>,
}

#[vtable]
impl MiscDevice for Notes {
    /// Что лежит в `file->private_data`: здесь — закреплённый KBox.
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        KBox::try_pin_init(
            try_pin_init!(Notes { buf <- new_mutex!(KVVec::new(), "ch29_notes::buf") }),
            GFP_KERNEL,
        )
    }
    // ANCHOR_END: open

    // ANCHOR: read
    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let me = kiocb.file();
        let buf = me.buf.lock();
        if *module_parameters::simple.value() == 1 {
            return iov.simple_read_from_buffer(kiocb.ki_pos_mut(), &buf);
        }
        let pos = usize::try_from(kiocb.ki_pos()).map_err(|_| EINVAL)?;
        let rest = buf.get(pos..).unwrap_or(&[]);
        if rest.is_empty() || iov.is_empty() {
            return Ok(0);
        }
        let n = iov.copy_to_iter(rest);
        if n == 0 {
            // Есть что отдать и есть куда, а не скопировано ни байта:
            // адрес пользователя недоступен.
            return Err(EFAULT);
        }
        *kiocb.ki_pos_mut() += n as i64;
        Ok(n)
    }
    // ANCHOR_END: read

    // ANCHOR: write
    fn write_iter(kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let me = kiocb.file();
        let mut buf = me.buf.lock();
        let len = iov.len();
        if buf.len() + len > CAPACITY {
            return Err(ENOSPC);
        }
        let n = iov.copy_from_iter_vec(&mut buf, GFP_KERNEL)?;
        if n == 0 && len > 0 {
            return Err(EFAULT);
        }
        Ok(n)
    }
    // ANCHOR_END: write

    // ANCHOR: ioctl_fn
    fn ioctl(me: Pin<&Notes>, _file: &File, cmd: u32, arg: usize) -> Result<isize> {
        match cmd {
            NOTES_LEN => {
                let len = me.buf.lock().len() as u64;
                let mut out = UserSlice::new(UserPtr::from_addr(arg), size_of::<u64>()).writer();
                out.write(&len)?;
                Ok(0)
            }
            NOTES_CLEAR => {
                me.buf.lock().clear();
                Ok(0)
            }
            _ => Err(ENOTTY),
        }
    }
    // ANCHOR_END: ioctl_fn
}

#[pinned_drop]
impl PinnedDrop for Notes {
    fn drop(self: Pin<&mut Self>) {
        pr_info!("закрыт, в буфере {} байт\n", self.buf.lock().len());
    }
}
