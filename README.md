# Rust для системного программиста

Учебник Rust для студентов, которые знают C и знакомы с устройством ОС.
Язык разбирается на уровне механизмов: как устроены типы в памяти, во что
компилируется код, где проходит граница `unsafe`. Каждое утверждение
книги проверяется кодом из этого репозитория: тестами, Miri, листингами
ассемблера для x86_64 и ARM Cortex-M и прошивками в QEMU.

В каждой главе есть задачи от простых к сложным, разборы всех решений
открыто лежат в приложении E. Через всю книгу проходит сквозной проект
«станция телеметрии»: прошивка Cortex-M собирает измерения, хост на Linux
их принимает.

## Состояние

- Программа курса: глава 0, 31 глава в шести частях, приложения A–E.
- Готова эталонная глава 6 «Составные типы и раскладка памяти»:
  11 разделов, задачи A–D, первая задача проекта П6 и разборы в
  приложении E. Остальные главы, кроме 0–29, пока содержат план из
  программы курса.
- Готовы главы 0–2: окружение и инструменты (rustup, cargo, чтение
  ассемблера, одна программа на x86_64 и Cortex-M, Miri и clippy, QEMU и
  gdb), скалярные типы и их представление в памяти, выражения и
  сопоставление с образцом вместе с тем, во что компилируется `match`.
  В каждой главе задачи A–D (в главе 1 — A, B1, B2, C, D), эталонные
  решения и разборы в приложении E.
- Готовы главы 3–5: владение и перемещение (drop и его флаги, перемещение
  в машинном коде, стек прошивки и стражи прерываний на Cortex-M),
  заимствование и ссылки (правило читателей и писателя, NLL, раздельные
  заимствования, `noalias` и границы проверки, Polonius), времена жизни
  (элизия, разбор без копирования, `'static` во flash, вариантность,
  `for<'a>`, GAT, захват в `impl Trait`). Задачи A–D, решения и разборы
  в приложении E, прошивки в `firmware/ch03-*` и `firmware/ch05-addr`.
- Готова глава 7 «Обработка ошибок и паника»: `Result` и `?` в машинном
  коде, `From` и `Box<dyn Error>`, цепочки `source`, размер ошибки,
  паника и раскрутка по таблицам `.eh_frame`, граница FFI и
  `extern "C-unwind"`, `#[panic_handler]` на Cortex-M. Задачи A–D,
  второй шаг проекта П7 (ошибки протокола), прошивки
  `firmware/ch07-panic` и `firmware/ch07-frame`.
- Готова глава 8 «Модули, крейты и сборка»: видимость и что она делает
  с символами, крейты и `Cargo.lock`, `cfg` и `target_has_atomic` на
  Cortex-M0, аддитивность возможностей, `build.rs` на хосте и для цели,
  `rlib`, `staticlib` и `cdylib` изнутри, манглинг v0 и legacy,
  столкновения имён без манглинга, секции и `.uninit`. Задачи A–D
  (видимость, таблица синуса для M0, staticlib для программы на C с
  разбором имён, неаддитивные возможности), прошивка
  `firmware/ch08-sections`.
- Готова глава 9 «Трейты»: реализации и статическая диспетчеризация в
  машинном коде против таблицы указателей на функции в C,
  ассоциированные типы и константы (регистры SysTick как типы с
  проверкой выравнивания при мономорфизации), супертрейты, правило
  сироты и когерентность, маркерные трейты и `unsafe impl Send`,
  операторы Q15 с DSP-инструкциями Cortex-M4, `From`, `AsRef`, `Borrow`,
  `derive` изнутри, запечатанные трейты и трейты-расширения. Задачи A–D
  (шесть пар реализаций, регистры PL011 как типы, запечатанные выводы
  GPIO и битовые поля, драйвер датчика для I2C и SPI поверх
  `embedded-hal` 1.0), прошивка `firmware/ch09-systick`.
- Готова глава 10 «Обобщения и мономорфизация»: `qsort` с компаратором
  через указатель и сортировка, размноженная макросом, против обобщений
  с ограничениями, `where` и условные методы, `impl Trait` в аргументе,
  результате и методах трейтов, параметры-константы (маска вместо
  деления, проверки при мономорфизации), копии в бинарнике по `nm` и
  `cargo bloat`, слияние копий и внутренняя необобщённая функция,
  встраивание замыкания против указателя на функцию, typestate для UART
  PL011 без единой лишней инструкции. Задачи A–D (сколько копий в шести
  программах, `qsort` против `sort_unstable_by` на 10 миллионах чисел,
  матрицы с размерностями в типе, typestate-драйвер SPI-flash W25Q
  поверх `embedded-hal` 1.0), прошивка `firmware/ch10-typestate`.
- Готова глава 11 «Динамическая диспетчеризация»: `struct
  file_operations` и цена вызова через таблицу на C, толстые указатели и
  их время жизни, содержимое таблицы методов в ассемблере x86-64 и
  Cortex-M, `drop` и освобождение `Box<dyn>`, таблицы во flash,
  совместимость с `dyn` и `where Self: Sized`, `?Sized` и пересылающие
  реализации, `Rc<dyn>`, `&mut dyn FnMut` без кучи, `Any` и приведение к
  супертрейту, предсказатель косвенных переходов, девиртуализация и
  retpoline, перечисление вместо `dyn`. Задачи A–D (размеры указателей,
  таблица методов вручную, `dyn` против перечисления и обобщений на 2, 8
  и 64 типах, размер из таблицы под Miri), прошивка `firmware/ch11-dyn`.
- Готова глава 12 «Замыкания»: реестр обработчиков с `void *ctx` на C и
  его беды под ASan, три способа захвата и точный захват полей,
  замыкание как структура и его размер, `Fn`, `FnMut`, `FnOnce` и
  пересылка через `&mut F`, типы функций и указатели на них, таблица
  обработчиков во flash и бит Thumb, трамплин для `qsort_r` и паника в
  нём, вложенные функции GCC и исполняемый стек, `impl Fn` и `use<>` в
  результате, окружение в регистрах и встраивание против `&mut dyn
  FnMut` в ассемблере, `Box<dyn FnOnce>`, автотрейты замыканий и их
  утечка через `impl Trait`, асинхронные замыкания. Задачи A–D (размеры
  восьми замыканий, реестр обработчиков без кучи двумя способами,
  комбинаторы парсеров для ответов AT-команд без косвенных вызовов,
  ошибка `Send` далеко от причины), третий шаг проекта П12 (разбор кадра
  комбинаторами), прошивка `firmware/ch12-closures`.
- Готова глава 13 «Итераторы»: циклы по индексам на C с ошибками на
  единицу и на знак, `Iterator` и `next`, три формы `for` и
  `IntoIterator`, итератор среза в ассемблере, `by_ref` и диапазоны,
  ленивость и адаптеры, поиск с ранним выходом, `try_fold` и
  `ControlFlow`, `collect` в разные коллекции и в `Result`, свой
  итератор по кольцевому буферу с `DoubleEndedIterator`,
  `ExactSizeIterator` и своей свёрткой, `size_hint`, `from_fn` и
  `successors`, внешняя и внутренняя итерация по `chain` в ассемблере,
  проверки границ и `assert!` перед циклом, SSE2, AVX2 с выбором во время
  работы и DSP-инструкции Cortex-M4, `chunks_exact`, ловушки `rev` и
  `enumerate`, `take_while`, `zip` и `sum`. Задачи A–D (ленивость восьми
  цепочек, итераторы по битам, простым числам и кадрам SLIP, скалярное
  произведение `i32` и `f32` в ассемблере, адаптер, который ломает
  векторизацию, и аллокации `collect`), четвёртый шаг проекта П13
  (итератор по кадрам в потоке байт), прошивка `firmware/ch13-iter`.
- Готова глава 14 «Умные указатели и внутренняя изменяемость»: `Box` и
  его размеры, рекурсивные типы, `leak`, `into_raw` и `from_raw`, большой
  буфер без стека; `Deref` и `DerefMut` на тени регистров; `Rc` и `Weak`
  — блок со счётчиками, `clone` и `drop` в ассемблере, подписки на
  `Weak`, `new_cyclic`, циклы, `make_mut`; `Arc` — `lock inc` на x86-64,
  `ldrex`/`strex` и барьеры `dmb` на Cortex-M3, отсутствие на Cortex-M0;
  `UnsafeCell`, `Cell` и `RefCell` и что они меняют в коде; `OnceCell`,
  `LazyCell`, `OnceLock` и `LazyLock`; `Cow`; ловушки временных значений
  в `match` и `for`, `Rc<[T]>` против `Rc<Vec<T>>`, срез ячеек,
  `Arc::into_inner`. Задачи A–D (паника `RefCell` и счётчики `Rc` в
  двенадцати фрагментах, свой `Rc` со слабыми ссылками под Miri,
  двусвязный список на `Rc` и на арене, цена `Arc::clone` из многих
  потоков и ложное разделение). Проектного раздела у главы нет.
- Готова глава 15 «Коллекции и аллокация»: `Vec` как (ptr, cap, len),
  стратегия роста и `try_reserve`; `String` и `&str`, почему строка не
  индексируется; `HashMap` — SwissTable, SipHash и защита от HashDoS,
  быстрые хешеры; `BTreeMap`, `VecDeque`, `BinaryHeap` и отсортированный
  `Vec`; локальность `Vec<T>` против `Vec<Box<T>>` и `LinkedList`;
  `#[global_allocator]`, `Layout`, арены; переход `malloc` на `mmap` под
  `strace`; коллекции `heapless` и фрагментация кучи на Cortex-M; `retain`,
  `drain`, `entry`. Задачи A–D (предсказать рост `Vec`, аллокатор-счётчик,
  bump-арена для дерева, своя таблица с открытой адресацией и длиной
  пробы), шаг проекта П15 (хранилище измерений и аллокатор-счётчик),
  прошивка `firmware/ch15-heap`.
- Готова глава 16 «unsafe и неопределённое поведение»: пять возможностей
  `unsafe`; инварианты валидности и безопасности и перечень UB; что
  оптимизатор делает с проверкой границ и `unreachable_unchecked`
  (ассемблер); provenance и строгий API; модели алиасинга Stacked и Tree
  Borrows и Miri; как запереть `unsafe` за безопасным API; гигиена и
  edition 2024. Задачи A–D (каталог UB под Miri, `split_first_mut`
  вручную, unsound-API с use-after-free, расхождение Stacked и Tree
  Borrows). Проектного раздела у главы нет.
- Готова глава 17 «Низкоуровневая работа с памятью»: `MaybeUninit` и сборка
  значения по частям; операции с сырыми указателями (`read`/`write`,
  `copy_nonoverlapping`, `swap`, невыровненное чтение); `NonNull` и
  `PhantomData`; `transmute` и его безопасные замены, `offset_of!`, типы
  нулевого размера; `volatile` и MMIO с `compiler_fence` (ассемблер
  подтверждает, что обращения не сливаются); типы динамического размера с
  хвостовым массивом. Задачи A–D (каталог `transmute`: корректные, UB под
  Miri, не компилирующиеся; разбор заголовка ELF64 без копирования и через
  `from_le_bytes`; тегированный указатель со строгим provenance; свой DST
  против `Box<[T]>`). Проектного раздела у главы нет.
- Готова глава 18 «FFI с C»: блок `unsafe extern "C"` и соглашение о
  вызовах; `#[repr(C)]`/`repr(transparent)` и структуры по значению; строки
  `CStr`/`CString`, литерал `c"..."`, не-UTF-8 из C; владение через границу
  (`into_raw`/`from_raw`) и экспорт Rust в C непрозрачным описателем;
  обратные вызовы с переходником и защитой от паники (`catch_unwind`);
  обёртка над zlib и инструменты (`bindgen`, `cbindgen`, `cc`). То, что C
  вызывает Rust, проверяет C-программа под AddressSanitizer. Задачи A–D
  (каталог из шести объявлений libc; `qsort_r` из glibc с замыканием;
  кольцевой буфер за непрозрачным описателем с драйвером на C; безопасная
  обёртка над zlib `compress`/`uncompress`). Глава хостовая: под Miri и
  Cortex-M не идёт.
- Готова глава 19 «Встроенный ассемблер, интринсики и SIMD»: `asm!` с
  операндами и опциями; `global_asm!` и naked-функции; интринсики
  `core::arch` и `#[target_feature]`; выбор реализации во время выполнения;
  переносимый SIMD (`core::simd`, nightly); инструкции управления ядром
  Cortex-M. Обещанные инструкции закреплены тестом снятого ассемблера.
  Задачи A–D (три примитива на `asm!`, функция целиком в `global_asm!`,
  поиск байта на SIMD с диспетчеризацией, кооперативная сопрограмма).
  Глава в основном хостовая (x86-64); модуль управления ядром собирается
  под три цели Cortex-M.
- Готова глава 20 «Потоки, Send и Sync»: `spawn`, `join` и `move`; `Send` и
  `Sync` как свойства типов; общее состояние через `Arc`, `Mutex` и
  `RwLock`; области потоков `thread::scope`; каналы и ожидание; цена
  ложного разделения кэш-линии. Задачи A–D (`SendBox` с ручным `unsafe impl
  Send`, параллельный подсчёт слов, мьютекс на трёх состояниях поверх
  `futex`, перевод между счетами без взаимной блокировки). Глава хостовая.
- Готова глава 21 «Атомики и модель памяти»: атомарные типы и
  `compare_exchange(_weak)`; happens-before через Release/Acquire;
  `SeqCst`, заборы и буфер записи; во что порядки компилируются на x86 и
  ARM; lock-free очередь SPSC на атомарных индексах (собирается и для
  Cortex-M0); спинлок, ticket lock и конвой, ABA и тегированный указатель.
  Задачи A–D (спинлок, справедливый ticket lock, проба на буфер записи,
  стек Трайбера против ABA).
- Готова глава 22 «Асинхронность изнутри»: `Future`, `poll` и `Waker`
  вручную; `async fn` как конечный автомат и его размер; `Pin`, `Unpin` и
  самоссылки; исполнитель с очередью, реактор на `epoll` и модель
  завершения io_uring; `select`, `join` и отмена через `drop`, отменобезопасность;
  `async` на Cortex-M со сном в `wfi`. Задачи A–D2 (размеры шести
  автоматов, исполнитель с таймерами на `BinaryHeap`, реактор на `epoll`
  и эхо-сервер на тысячу соединений, исполнитель без кучи для Cortex-M,
  отменобезопасное чтение кадров), прошивки `firmware/ch22-async` и
  `firmware/ch22-tasks`.
- Готова глава 23 «Декларативные макросы»: правила и виды фрагментов,
  `expr` против `tt` и отказ без отката; повторения и подсчёт аргументов
  при компиляции; гигиена, элементы без гигиены, пути от `::core` и
  `$crate`; внутренние правила, TT-muncher и предел рекурсии; текстовая
  видимость и `#[macro_export]`; X-macro, таблица прерываний, регистры и
  `unsafe` внутри макроса, сверка ассемблера раскрытого и рукописного кода.
  Задачи A–D (шесть раскрытий, флаги регистра в духе bitflags, описание
  периферии с правами доступа в типах, конечный автомат на TT-muncher и
  его граница с процедурным макросом), прошивка `firmware/ch23-periph`.
- Готова глава 24 «Процедурные макросы»: крейт `proc-macro` и дерево
  токенов, `proc_macro` против `proc_macro2`; свой derive на syn и quote
  с обобщениями и полными путями; спаны, `syn::Error` и все ошибки за
  одну сборку, const-проверка размера со спаном, гигиена `call_site` и
  `mixed_site`; атрибуты-помощники и путь к переименованному крейту;
  `#[interrupt]` в духе cortex-m-rt с проверкой сигнатуры и имени
  прерывания; модульные тесты, снимки trybuild и раскрытия, Miri; где и
  почём выполняется макрос (на хосте, время сборки syn, dep-info через
  `include_bytes!`). Задачи A–D (построчное раскрытие `#[derive(Debug)]`
  и `#[entry]`, `#[derive(Bitfield)]` со спанами ошибок, `#[timed]` на
  SysTick и DWT, `include_svd!` по файлу CMSIS-SVD и сравнение со
  svd2rust), прошивки `firmware/ch24-expand` и `firmware/ch24-tasks`.
- Готова глава 25 «Вычисления при компиляции»: `const` как значение и
  `static` как объект (образцы, повтор в массиве, продвижение в
  `'static` и адреса копий), `const fn` с её ограничениями и паникой как
  E0080 (делитель UART PL011), проверки при компиляции — раскладка
  `repr(C)`, инвариант таблицы, обобщённые проверки и разница между
  `cargo check` и `cargo build`; атомики, `static mut` в редакции 2024,
  `Mutex::new` и `LazyLock` в `static`; таблица обратных вместо деления на
  Cortex-M0 и `hex!` для байтов кадра; цена в ОЗУ на прошивке
  `firmware/ch25-flash`. Шаг проекта — таблица CRC-16 при компиляции и
  наблюдение, что LLVM 22 строит такую таблицу сам при `opt-level = 3`.
  Задачи A–D (секции шести объявлений, CRC-32 и синус без `build.rs`,
  `ip!` с ошибкой сборки, бюджет ОЗУ из `memory.x`).
- Готова глава 26 «no_std: программа без операционной системы»: слои
  `core`, `alloc` и `std` и ошибки, которыми о них говорит компилятор
  (`std` и атомарный RMW на Cortex-M0, распределитель для `alloc`);
  процесс Linux x86-64 без libc со своим `_start`, разбором argc, argv и
  auxv, системными вызовами через `syscall`, LTO против
  `rust_eh_personality` и сдвинутым стеком; прошивка без cortex-m-rt —
  таблица векторов на Rust, свой скрипт компоновки с проверкой `.data` и
  `.bss`, semihosting без крейтов; откуда берутся `memcpy`, `memset` и
  `strlen` и почему своя `memset` зовёт саму себя (и в GCC 13 тоже);
  тесты `no_std` на хосте. Шаг проекта — станция на своей среде
  выполнения (`firmware/ch26-station`). Задачи A–D (где живут десять
  элементов, ELF на 376 байт, свой код запуска с `.data`, `.bss`,
  `.uninit` и FPU в `firmware/ch26-boot`, свои `memcpy` и родня с
  `#![no_builtins]` и прошивкой `firmware/ch26-tasks`).
- Готова глава 27 «Cortex-M на Rust»: режимы Thread и Handler, MSP и
  PSP, кадр исключения и EXC_RETURN на SVCall; NVIC — байт приоритета,
  вытеснение, BASEPRI и HardFault FORCED от `svc` под маской; PAC из
  SVD через svd2rust 0.37.1 с проверкой в CI, безопасные и `unsafe`
  поля; владение периферией и typestate выводов, bit-band;
  embedded-hal 1.0 и embedded-io — драйвер 74HC595, UART, таймер, АЦП и
  ассемблер без вызовов; прерывания с `Mutex<RefCell<Option>>`,
  несброшенный запрос и долгая критическая секция; HardFault по
  испорченному указателю, сторож стека на MPU и flip-link; RTIC 2 с
  потолками BASEPRI. Шаг проекта — станция на HAL с АЦП по таймеру,
  кадры которой из UART0 разбирает тест на хосте (`firmware/ch27-station`).
  Задачи A–D2 (таблица векторов из ELF, байт в UART0 тремя способами,
  консоль команд с эхом, переключатель задач на PendSV, разбор CFSR и
  HFSR с продолжением после сбоя; прошивки — `firmware/ch27-tasks`).
- Готова глава 28 «Системное программирование под Linux»: один вызов
  через std, libc, rustix и nix, что std делает до `main`, цена вызова и
  vDSO; `OwnedFd`, `BorrowedFd` и `AsFd`, копия файла на C с `goto` под
  GCC, clang и valgrind, проверка двойного закрытия в отладочной сборке,
  memfd; `Command` через posix_spawn и через fork с `pre_exec`, `fork`
  под чужим мьютексом, наследование маски сигналов; SIGPIPE, обработчик-
  флаг, `println!` в обработчике, EINTR и SA_RESTART, signalfd; `mmap`,
  страничные сбои, огромные страницы, SIGBUS; epoll и обзор io_uring;
  разбор /proc/self/maps и status. Шаг проекта — журнал кадров на mmap,
  переживающий SIGKILL, и демон с signalfd и epoll. Задачи A–D (системные
  вызовы шести операций под strace, `ls -la` на getdents64, мини-оболочка
  с группами процессов и Ctrl-C на псевдотерминале, поиск в файле на
  2 ГиБ через read и mmap с огромными страницами).
- Готова глава 29 «Rust в ядре Linux»: два ядра 7.0 с `CONFIG_RUST`
  (обычное и отладочное с KASAN, kmemleak и lockdep), сборка из
  tinyconfig, init на Rust в initramfs и QEMU; привязки, помощники и
  крейт `kernel`; `module!`, параметры, ошибки как `Result` и откат без
  `goto`; `KBox`, `KVec`, флаги GFP, сон под спин-блокировкой, стек ядра
  и запрет плавающей точки; pin-init и мьютекс на месте; `SpinLock`,
  `Mutex`, атомики LKMM и `Arc` в работах очереди против гонки на C;
  misc-устройство и граница с пространством пользователя с ошибкой
  `simple_read_from_buffer` в 7.0; драйвер PCI-устройства QEMU `edu` с
  MMIO, MSI и проверкой границ регистров при компоновке; паника как
  oops без раскрутки, `decode_stacktrace.sh` и модуль в gdb. Задачи A–D2
  (шесть ошибок драйвера на отладочном ядре, параметр модуля, счётчик
  против эталона на C, `kobject-example` через привязки, путь `read`
  Rust против C по трассе ftrace и ассемблеру).
- Готовы приложения A–D: словарь соответствий C → Rust, edition 2024 для
  системного кода, чек-лист ревью `unsafe` с разбором примера, установка
  окружения и отладка в QEMU. Их код лежит в `code/appendix-*`.
- Приложение F — глоссарий: термин, английский оригинал, определение и
  раздел, где термин введён. Во всей книге — введении, главах, проекте,
  приложениях и разборах задач — каждый термин определён в тексте, а
  первое упоминание в главе ведёт в глоссарий.

## Чтение

```console
$ mdbook serve --open                 # книга на http://localhost:3000
$ mdbook build                        # статический сайт в каталоге book/
```

## Устройство репозитория

```text
src/                      текст книги (mdBook)
  ch00/ ch01/ ch02/       главы 0–2, по файлу на раздел
  ch03/ ch04/ ch05/       главы 3–5, по файлу на раздел
  ch06/                   эталонная глава, по файлу на раздел
  ch07/ ch08/ ch09/       главы 7–9, по файлу на раздел
  ch10/ ch11/ ch12/       главы 10–12, по файлу на раздел
  ch13/ ch14/ ch15/        главы 13–15, по файлу на раздел
  ch16/ ch17/ ch18/        главы 16–18, по файлу на раздел
  ch19/ ch20/ ch21/ ch22/  главы 19–22, по файлу на раздел
  ch23/                   глава 23, по файлу на раздел
  ch24/                   глава 24, по файлу на раздел
  ch25/                   глава 25, по файлу на раздел
  ch26/                   глава 26, по файлу на раздел
  ch27/                   глава 27, по файлу на раздел
  ch28/                   глава 28, по файлу на раздел
  ch29/                   глава 29, по файлу на раздел
  ch30/                   глава 30, по файлу на раздел, и flamegraph.svg раздела 30.3
  ch31/                   глава 31, по файлу на раздел
  appendix-e/             разборы задач всех глав
  appendix-f.md           глоссарий: статьи, образцы для поиска терминов, место определения
code/chNN/                примеры главы; листинги попадают в текст через {{#include}}
code/ch01/c/ … ch15/c/    те же примеры на C и run.sh, который сверяет их вывод с текстом
code/ch0[45]/rejected/    программы, которые проверка отвергает; их собирают тесты
code/ch0[45789]/fragments/  фрагменты к задачам A глав 4, 5, 7, 8 и 9
code/ch10/fragments/      шесть программ задачи 10.A: сколько копий обобщённой функции
code/ch10/tests/          замеры кода для Cortex-M3: typestate и внутренняя функция
code/ch11/tests/          ассемблер вызовов через таблицу; на nightly — DynMetadata и retpoline
code/ch12/tests/          ассемблер окружения замыканий и вызовов через dyn; на nightly — Fn вручную
code/ch12/examples/       калибровка тремя способами: обобщение, &mut dyn FnMut, указатель
code/ch13/tests/          ассемблер цепочек: chain, проверки границ, SSE2, AVX2, DSP Cortex-M4
code/ch13/examples/       время на элемент: for и fold, индексы и zip, chunks и chunks_exact
code/ch14/tests/          ассемблер Rc, Arc и ячеек, обращения к распределителю, UB под Miri
code/ch14/examples/       Box::new на 16 МиБ, запись через &u32, цена clone и ячеек
code/ch15/tests/          ассемблер push, VecDeque::get и heapless, блоки SwissTable и BTreeMap
code/ch15/examples/       рост Vec под strace, раскладка SwissTable, вырождение таблицы, локальность
code/ch16/tests/          ассемблер проверки границ, get_unchecked и unreachable_unchecked
code/ch16/ub/             run.sh: каталог UB под Miri, unsound-API и расхождение Stacked/Tree Borrows
code/ch17/tests/          ассемблер: volatile-чтения не сливаются, обычные — сливаются
code/ch17/ub/             run.sh: UB-вызовы каталога transmute под Miri (задача A)
code/ch18/c/              support.c (вызывается из Rust) и run.sh: C вызывает Rust под ASan
code/ch18/build.rs        компилирует support.c системным cc в статический архив
solutions/ch18/c/         драйвер кольцевого буфера на C и run.sh под ASan (задача C)
code/ch19/tests/          снятый ассемблер: обещанные инструкции (rdtsc, lfence, imul, paddd, vpaddd) эмитятся
code/ch19/examples/       emit_probe: драйвер для теста кодогенерации, зовёт asm!-примитивы
code/ch21/tests/          снятый ассемблер: SeqCst-store даёт xchg, атомарный RMW — префикс lock
code/ch21/examples/       emit_probe: драйвер для теста кодогенерации, зовёт атомарные операции
code/ch23/c/              macros.c: ловушки препроцессора и X-macro; run.sh сверяет вывод и предупреждения
code/ch23/expanded/       swap! с контекстами гигиены: исходник и снимок -Zunpretty=expanded,hygiene
code/ch23/fragments/      шесть вызовов макросов задачи 23.A
code/ch23/tests/          снимок раскрытия и ассемблер: макросная отправка байта — псевдоним рукописной
code/ch24-macros/         крейт proc-macro главы 24: Describe, #[interrupt], #[traced], разбор токенов
code/ch24/tests/          снимки trybuild (ui/), снимки раскрытия, макрос собран для хоста, путь к крейту
code/ch24/expanded/       снимки -Zunpretty=expanded: derive(Describe) и #[interrupt] для UART0
code/ch24/renamed/        отдельное рабочее пространство: ch24 под именем station и #[describe(crate = ...)]
code/ch24/svd/            lm3s6965.svd: два UART в формате CMSIS-SVD для include_svd! задачи 24.D
code/ch25/c/              const_size.c, constexpr.c и divisor.c: C17, C23 и препроцессор; run.sh сверяет сообщения gcc
code/ch25/fragments/      шесть объявлений задачи 25.A и when.rs: когда rustc вычисляет константу
code/ch25/tests/          ассемблер Cortex-M0/M3 (деление, CRC), check против build, кадр из hex!
code/ch26/c/              snprintf.c и memset.c: что приносит newlib и когда memset зовёт саму себя; run.sh
code/ch26/fragments/      программы, которые no_std отвергает: Vec без alloc, std на M0, Arc и fetch_add на M0
code/ch26/start/          отдельное рабочее пространство: процесс Linux x86-64 без libc со своим _start
code/ch26/tests/          _start под ядром, ассемблер вызовов memcpy на трёх целях, тексты ошибок no_std
code/ch27/svd/            lm3s6965.svd (SYSCTL, GPIO, UART, таймеры, АЦП) и gen-pac.sh для svd2rust 0.37.1
code/ch27/tests/          станция в QEMU: кадры из UART0 в файл, разбор крейтом telemetry-frame
code/ch27-pac/            PAC главы 27, вывод svd2rust; CI сверяет его с повторной генерацией
code/ch27-hal/            HAL главы 27: GPIO с typestate, UART, таймер, АЦП; tests/ — ассемблер и ошибки типов
code/ch28/c/              copy.c: копия файла с errno и goto, вариант с утечкой; run.sh — GCC, clang, valgrind
code/ch28/tests/          опыты главы отдельными процессами, strace-журналы, демон журнала под SIGKILL
code/ch29/kernel/         build.sh и фрагменты .config: ядра Linux 7.0 с CONFIG_RUST, обычное и отладочное
code/ch29/modules/        модули ядра главы на Rust и их пары на C; собирает Kbuild, а не cargo
code/ch29/modules-fail/   модули, которые не должны собраться: pin, Send/Sync, плавающая точка, границы MMIO
code/ch29/tests/          опыты главы в QEMU, сеанс gdb, .modinfo, разбор трассы oops, цитаты из ядра
code/ch30/src/bin/        программы опытов главы 30: часы, счётчики, профиль, матрица, проверки, память, станция
code/ch30/benches/        замеры criterion и divan, счётчик выделений divan
code/ch30/tests/          ассемблер, модель кэша и предсказателя cachegrind, профиль perf, время с запасом
code/ch30/pgo/            run.sh: PGO программы inlining и ассемблер check_plain с профилем и без
code/ch30-dep/            «чужой» крейт для опытов главы 30 со встраиванием между крейтами
code/ch31/src/            окно отсчётов, SLIP, команды хоста, очередь SPSC под loom, доказательства Kani
code/ch31/src/bin/san.rs  опыты санитайзеров и Miri: uaf, overflow, ffi, leak, race, spsc, relaxed, once
code/ch31/tests/          тесты главы 31: отчёт о падении, свой main, proptest, санитайзеры, loom
code/ch31/fuzz/           цели cargo-fuzz главы 31 (отдельный workspace) и run.sh с их исходами
code/ch31/c/              разбор команд на C: ASan, цели libFuzzer, CBMC; run.sh сверяет с текстом
code/ch31/{miri,kani,mutants,supply}/  run.sh: Miri, Kani, cargo-mutants и цепочка поставок
code/ch31/supply/build/   приложение с двумя зависимостями, чей код выполняется при сборке
code/ch31-irq/            логика прерывания АЦП без std: тесты хоста и прошивка firmware/ch31-tests
code/ch07/c-size/         программа задачи 7.C: размер при panic = "unwind" и "abort"
code/ch07/hir/            во что раскрывается `?`: снимок HIR для раздела 7.1
code/ch08/typo/ unify/ ffi/  отдельные рабочие пространства для опытов главы 8 с cargo
code/ch08/dup/            столкновение имён без манглинга: исходники для rustc
code/ch09/expanded/       что пишет derive: исходник и снимок -Zunpretty=expanded
code/ch06-regs/           драйвер UART PL011 из раздела 6.9
code/appendix-a/          словарь C → Rust; в c/ те же примеры на C и run.sh для UBSan
code/appendix-b/          edition 2024 для системного кода
code/appendix-b-2021/     те же исходники в edition 2021 для сравнения
code/appendix-c/          вектор фиксированной ёмкости до и после ревью unsafe
exercises/chNN/           заготовки задач
exercises/ch02-2021/      задача 2.D в edition 2021: тот же файл, что в exercises/ch02
exercises/ch08/c/         задача 8.C: программа на C, Makefile и крейт station
exercises/ch08/d-station/ задача 8.D: рабочее пространство с неаддитивными возможностями
exercises/project-frame/  заготовка задач П6, П7, П12 и П13
exercises/project-store/  заготовка задачи П15: хранилище измерений
solutions/chNN/           эталонные решения
solutions/ch02-2021/      решение задачи 2.D в edition 2021
solutions/ch08/c/ d-station/  решения задач 8.C и 8.D
{exercises,solutions}/ch26/tiny/  задача 26.B: программа без libc, отдельное рабочее пространство
{exercises,solutions}/ch31/fuzz/  цели фаззинга задачи 31.B, отдельные рабочие пространства
exercises/ch24-macros/    заготовки макросов задач 24.B–24.D; решения — в solutions/ch24-macros
tests-shared/             тесты, общие для заготовок и решений
tests-shared/ch24-ui/     снимки ошибок задач 24.B–24.D для trybuild, общие для заготовок и решений
tests-shared/ch28-ops.rs  программа задачи 28.A: шесть операций std между метками getppid
tests-shared/ch29/        фрагменты задачи 29.A на C и Rust, эталон счётчика задачи 29.C на C
tests-shared/ch29-guest.rs  init гостя задач главы 29: сценарии A–D2 в QEMU
tests-shared/ch30-task.rs  программа замеров задач главы 30: тесты зовут её напрямую и под cachegrind
project/                  сквозной проект «станция телеметрии»
firmware/                 прошивки для QEMU (отдельный workspace)
theme/                    стили врезок: справка, сравнение с C, под капотом, трюк, ловушка
tools/                    вспомогательные проверки для CI
deny.toml                 политика зависимостей для cargo-deny (раздел 31.9)
```

## Окружение

Установка на Arch Linux и Debian/Ubuntu описана в
[приложении D](src/appendix-d.md). Коротко:

- Rust 1.97.0 и три цели Cortex-M закреплены в `rust-toolchain.toml`,
  rustup поставит их сам;
- nightly с Miri для проверки `unsafe` и флагов `-Z`;
- `qemu-system-arm` 8.x, машины `lm3s6965evb` (Cortex-M3) и `microbit`
  (Cortex-M0), для главы 0 ещё `mps2-an386` (Cortex-M4F), для C-примеров
  главы 1 `mps2-an385` (Cortex-M3);
- `gdb-multiarch` для задачи 0.D, `llvm` (`llvm-objdump`, `llvm-nm`) для
  задачи 2.B, GCC и clang для C-примеров; `arm-none-eabi-gcc` с newlib
  нужен только для проверки C-примеров глав 1, 4 и 26 на Cortex-M;
- mdBook 0.5.4 и `cargo-show-asm`;
- для главы 27 — svd2rust 0.37.1 (повторная генерация PAC) и flip-link
  0.1.12 (опыт раздела 27.7; без него `run-demos.sh` его пропускает);
- для главы 29 — clang, lld и llvm 18, flex, bison, bc, libelf, cpio,
  компонент `rust-src` и bindgen-cli 0.72.1 для сборки ядра,
  `qemu-system-x86_64` 8.x и gdb для опытов; без собранного ядра
  (`bash code/ch29/kernel/build.sh`) тесты главы пропускаются;
- для главы 30 — valgrind 3.22 (cachegrind), perf из `linux-tools` (путь
  можно задать в `PERF`), компонент `llvm-tools` для PGO и inferno для
  flamegraph; без valgrind и perf их опыты пропускаются (опыты с perf —
  и при `kernel.perf_event_paranoid` выше 2), а `CH30_REQUIRE_VALGRIND`,
  `CH30_REQUIRE_PERF` и `CH30_REQUIRE_PERF_EVENTS` делают пропуск ошибкой;
- для главы 31 — nightly с компонентами miri и rust-src (санитайзеры
  собирают std с `-Zbuild-std`), llvm-symbolizer (пакет `llvm`: без него
  ASan печатает адреса вместо имён функций), cargo-fuzz 0.13.2, рантаймы
  compiler-rt для clang (пакет `libclang-rt-18-dev`), Kani 0.68.0
  (`cargo install --locked kani-verifier@0.68.0 && cargo kani setup`, он
  же ставит CBMC), cargo-mutants 27.1.0, cargo-deny 0.20.2, cargo-audit
  0.22.2, cargo-vet 0.10.3 и сеть для базы RustSec; без nightly опыты с
  санитайзерами, а без cargo-mutants прогон задачи D пропускаются, а
  `CH31_REQUIRE_SANITIZERS` и `CH31_REQUIRE_MUTANTS` делают пропуск
  ошибкой.

## Задачи

```console
$ cargo test -p ch00-exercises            # задачи A–C главы 0; D проверяет gdb
$ cargo test -p ch01-exercises            # все задачи главы 1
$ cargo test -p ch02-exercises            # все задачи главы 2
$ cargo test -p ch02-exercises-2021       # задача 2.D в edition 2021
$ cargo test -p ch03-exercises            # все задачи главы 3
$ cargo test -p ch04-exercises            # все задачи главы 4
$ cargo test -p ch05-exercises            # все задачи главы 5
$ cargo test -p ch06-exercises            # все задачи главы 6
$ cargo test -p ch06-exercises task_b     # одна задача
$ cargo test -p ch07-exercises            # все задачи главы 7
$ cargo test -p ch08-exercises            # все задачи главы 8
$ cargo test -p ch09-exercises            # все задачи главы 9
$ cargo test -p ch10-exercises            # все задачи главы 10
$ cargo run --release -p ch10-exercises --example b_bench   # время для задачи 10.B
$ cargo test -p ch11-exercises            # все задачи главы 11
$ cargo run --release -p ch11-exercises --example c_bench   # время для задачи 11.C
$ cargo test -p ch12-exercises            # все задачи главы 12
$ cargo test -p ch13-exercises            # все задачи главы 13
$ cargo test -p ch14-exercises            # все задачи главы 14
$ cargo run --release -p ch14-solutions --example lists        # замеры для задачи 14.C
$ cargo run --release -p ch14-solutions --example arc_threads  # замеры для задачи 14.D
$ cargo test -p ch15-exercises            # все задачи главы 15
$ cargo test -p project-store p15         # шаг проекта П15
$ cargo run --release -p ch15-solutions --example report       # аллокации решений прошлых глав
$ cargo run --release -p ch15-solutions --example ast          # дерево на Box против арены
$ cargo run --release -p ch15-solutions --example probes       # длина пробы и загрузка (задача 15.D)
$ cargo test -p ch16-exercises            # задачи B и C главы 16
$ bash code/ch16/ub/run.sh                # каталог UB, unsound-API и расхождение моделей (A, C, D)
$ cargo test -p ch17-exercises            # задачи B, C и D главы 17
$ bash code/ch17/ub/run.sh                # UB-вызовы каталога transmute (задача A)
$ cargo test -p ch18-exercises            # задачи A, B, D и логика FIFO задачи C главы 18
$ bash solutions/ch18/c/run.sh            # драйвер кольцевого буфера задачи C на C под ASan
$ cargo test -p ch19-exercises            # задачи A–D главы 19
$ cargo test -p ch20-exercises            # задачи A–D главы 20 (C — свой futex-мьютекс)
$ cargo test -p ch21-exercises            # задачи A–D главы 21 (спинлок, ticket, store buffering, Трайбер)
$ cargo test -p ch22-exercises            # задачи A–D2 главы 22 (C — реактор на epoll, только Linux x86-64)
$ cargo test -p ch23-exercises            # задачи A–D главы 23 (A сверяется с rustc -Zunpretty=expanded)
$ cargo test -p ch24-exercises            # задачи A–D главы 24 (ошибки макросов — снимки trybuild)
$ cargo test -p ch25-exercises            # задачи A–D главы 25 (D берёт бюджет ОЗУ из firmware/memory.x)
$ cargo test -p ch26-exercises            # задачи A, B и D главы 26 (C — прошивка, эталон firmware/ch26-boot)
$ cargo test -p ch27-exercises            # задачи A–D2 главы 27 (прошивки C, D1, D2 — образец firmware/ch27-tasks)
$ cargo test -p ch28-exercises            # задачи A–D главы 28 (A без strace пропускается)
$ bash code/ch29/kernel/build.sh          # ядра Linux 7.0 для главы 29: обычное и отладочное
$ cargo test -p ch29                      # опыты главы 29 в QEMU, gdb и модули, которые не собираются
$ cargo test -p ch29-exercises            # задачи A–D2 главы 29 (модули ядра — в exercises/ch29/modules)
$ cargo test -p ch30                      # опыты главы 30: ассемблер, cachegrind, perf, время
$ cargo test -p ch30-exercises            # задачи A–D главы 30 (замеры — программа ch30-task)
$ bash code/ch30/pgo/run.sh               # PGO: горячий путь подряд по профилю (раздел 30.5)
$ cargo test -p ch31-exercises            # задачи A–D главы 31 (D без cargo-mutants пропускается)
$ cd exercises/ch31/fuzz && cargo +nightly fuzz run packet   # фаззинг задачи 31.B, цели packet и config
$ cargo kani -p ch31-exercises            # доказательства задачи 31.C
$ cargo test -p project-frame             # задачи П6, П7, П12 и П13
$ cargo test -p project-frame p7          # только П7
$ cargo test -p project-frame p12         # только П12
$ cargo test -p project-frame p13         # только П13
```

Пока функции не реализованы, тесты падают на `todo!()`. Тесты общие для
заготовок и эталонных решений, поэтому зелёный прогон значит то же, что и
у автора.

## Проверки

Те же команды выполняет CI (`.github/workflows/ci.yml`):

```console
$ sudo apt-get install -y zlib1g-dev      # глава 18 линкуется с zlib (libz.so)
$ cargo fmt --all --check && (cd firmware && cargo fmt --all --check)
$ cargo clippy --workspace --all-targets -- -D warnings
$ cargo test                              # примеры глав, решения, проект
$ cargo test -p appendix-a --release      # часть поведения приложения A зависит от профиля
$ cargo +nightly miri test -p ch00 -p ch01 -p ch02 -p ch00-solutions -p ch01-solutions \
      -p ch02-solutions -p ch02-solutions-2021
$ cargo +nightly miri test -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions \
      -p ch05-solutions                   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch06 -p ch06-regs -p ch06-solutions -p telemetry-frame
$ cargo +nightly miri test -p ch07 -p ch07-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch08 -p ch08-solutions
$ cargo +nightly miri test -p ch09 -p ch09-solutions
$ cargo +nightly miri test -p ch10 -p ch10-solutions
$ cargo +nightly miri test -p ch11 -p ch11-solutions
$ cargo +nightly miri test -p ch12 -p ch12-solutions
$ cargo +nightly miri test -p ch13 -p ch13-solutions
$ cargo +nightly miri test -p ch14 -p ch14-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch15 -p ch15-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch16 -p ch16-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ bash code/ch16/ub/run.sh                # каталог UB главы 16 под Miri (задачи A, C, D)
$ cargo +nightly miri test -p ch17 -p ch17-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ MIRIFLAGS=-Zmiri-strict-provenance cargo +nightly miri test -p ch16 -p ch17 -p ch17-solutions
$ bash code/ch17/ub/run.sh                # UB-вызовы каталога transmute главы 17 под Miri (задача A)
$ cargo +nightly miri test -p appendix-a -p appendix-b -p appendix-b-2021 -p appendix-c
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p appendix-c
$ cargo +nightly miri test -p ch20 -p ch20-solutions -- shared_pointer a_sendbox
$ cargo +nightly miri test -p ch21 -- single_thread_fifo
$ cargo +nightly miri test -p ch22 -p ch22-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch23 -p ch23-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch24 -p ch24-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch25 -p ch25-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch26 -p ch26-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch27 -p ch27-hal -p ch27-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly test --doc -p ch00 -p ch01 -p ch02 -p ch06 -p ch07 -p ch08 -p ch09 \
      -p ch10 -p ch11 -p ch12 -p ch13 -p ch14 -p ch15 -p ch16 -p ch17 -p ch17-solutions \
      -p ch18 -p ch19 -p ch20 -p ch22 -p ch23 -p ch23-solutions -p ch24 -p ch25 \
      -p appendix-a -p appendix-b -p appendix-b-2021 -p appendix-c
$ cargo +nightly test -p ch11 --features nightly --test nightly   # DynMetadata и -Zretpoline
$ cargo +nightly test -p ch12 --features nightly --test nightly   # Fn, FnMut и FnOnce вручную
$ cargo +nightly test -p ch19 --features nightly --lib            # переносимый SIMD core::simd
$ cargo +nightly test --doc -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions \
      -p ch05-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch06-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch00-solutions -p ch01-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch02-solutions -- --skip task_a
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch03 -p ch04 -p ch05 \
      -p ch05-solutions -- --skip polonius
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch07 -p ch07-solutions \
      -- --skip task_a --skip task_c --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch08 -p ch08-solutions \
      -- --skip cargo_ --skip task_a --skip task_b_flash --skip task_c_from_c --skip task_d
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch09 -p ch09-solutions \
      -- --skip rustc_ --skip task_a --skip task_b_access --skip task_b_thumb --skip task_c_sealed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch10 -p ch10-solutions \
      -- --skip rustc_ --skip task_a --skip task_b_inlined --skip task_c_types --skip task_c_sizes \
      --skip task_d_types
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch11 -p ch11-solutions \
      -- --skip rustc_ --skip task_a_thumb --skip task_b_same_code --skip task_c_calls
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch12 -p ch12-solutions \
      -- --skip rustc_ --skip task_a_thumb --skip task_c_no_indirect_calls --skip task_d_far_away \
      --skip task_d_promise --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch13 -p ch13-solutions \
      -- --skip rustc_ --skip observed --skip task_c_vectorized --skip task_d_vectorization
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch14 -p ch14-solutions \
      -- --skip rustc_ --skip cargo_ --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch15 -p ch15-solutions \
      -- --skip rustc_ --skip cargo_ --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch16 -p ch16-solutions \
      -- --skip rustc_
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch17 -p ch17-solutions \
      -- --skip rustc_
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch19 -p ch19-solutions \
      -- --skip chapter_instructions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch22 -p ch22-solutions \
      -- --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch23 -p ch23-solutions \
      -- --skip rustc_ --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch24 -p ch24-macros \
      -p ch24-solutions -- --skip rustc_ --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch25 -p ch25-solutions \
      -- --skip rustc_ --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch26 -p ch26-solutions \
      -- --skip rustc_
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch27 -p ch27-hal \
      -p ch27-solutions -- --skip rustc_
$ cargo build -p ch06 -p ch07 --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch08 -p ch08-solutions --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch09 --no-default-features --features alloc --target thumbv6m-none-eabi
$ cargo build -p ch09-solutions -p ch09-exercises --target thumbv6m-none-eabi
$ cargo build -p ch10 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch10-solutions -p ch10-exercises --target thumbv6m-none-eabi
$ cargo build -p ch11 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch11-solutions -p ch11-exercises --target thumbv6m-none-eabi
$ cargo build -p ch12 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch12-solutions -p ch12-exercises --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch13 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch13-solutions -p ch13-exercises --no-default-features --target thumbv6m-none-eabi \
      # и с --features alloc
$ cargo build -p ch14 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch14-solutions -p ch14-exercises --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch15 --no-default-features --features alloc --target thumbv6m-none-eabi
$ cargo build -p ch17 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc; на трёх целях Cortex-M
$ cargo build -p ch19 -p ch21 --no-default-features --target thumbv6m-none-eabi   # wfi/PRIMASK и SPSC
$ cargo build -p ch22 -p ch22-solutions -p ch22-exercises --no-default-features \
      --target thumbv6m-none-eabi       # async без кучи; на трёх целях Cortex-M
$ cargo build -p ch23 --no-default-features --target thumbv6m-none-eabi   # код из макросов; на трёх целях
$ cargo build -p ch23-solutions -p ch23-exercises --target thumbv6m-none-eabi
$ cargo build -p ch24 --no-default-features --target thumbv6m-none-eabi   # макрос — для хоста; на трёх целях
$ cargo build -p ch24-solutions -p ch24-exercises --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch25 --no-default-features --target thumbv6m-none-eabi   # таблицы и проверки; на трёх целях
$ cargo build -p ch25-solutions -p ch25-exercises --target thumbv6m-none-eabi
$ cargo build -p ch26 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc; на трёх целях
$ cargo build -p ch26-solutions -p ch26-exercises --target thumbv6m-none-eabi
$ cargo build -p ch27 -p ch27-pac -p ch27-hal -p ch27-solutions -p ch27-exercises \
      --target thumbv6m-none-eabi       # на трёх целях; прошивки ch27-* — только Cortex-M3
$ (cd firmware && cargo clippy --workspace --target thumbv7m-none-eabi -- -D warnings)   # и под thumbv7em;
      # под thumbv6m — с --exclude для каждой прошивки ch27-*
$ bash code/ch27/svd/gen-pac.sh && git diff --exit-code code/ch27-pac   # PAC совпадает с выводом svd2rust
$ cargo build -p ch00 -p ch01 -p ch02 -p ch00-solutions -p ch01-solutions --target thumbv6m-none-eabi
$ cargo build -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions -p ch05-solutions \
      --target thumbv6m-none-eabi
$ bash firmware/run-demos.sh              # опыты глав 0, 3, 5–13, 15, 22–27 в QEMU: 6, 5, 2, 8, 11, 7, 3, 2, 5, 7, 5, 7, 7, 4, 7, 4, 18 и 21
$ cargo test -p ch27 --test station -- --ignored   # станция главы 27 в QEMU, кадры разбирает хост
$ cargo test -p ch02-solutions -- --ignored own_firmware   # задача 2.B на всех прошивках
$ bash code/appendix-a/c/run.sh           # C-примеры приложения A под UBSan
$ bash code/ch01/c/run.sh                 # C-примеры главы 1 на x86_64 и в QEMU
$ bash code/ch02/c/run.sh                 # switch из введения к главе 2
$ bash code/ch03/c/run.sh                 # queue.c из введения к главе 3: ASan и анализаторы
$ bash code/ch04/c/run.sh                 # samples.c, count.c и overlap.c из главы 4
$ bash code/ch05/c/run.sh                 # config.c из введения к главе 5
$ bash code/ch07/c/run.sh                 # errno.c, unchecked.c и longjmp.c из главы 7
$ bash code/ch08/c/run.sh                 # ifdef.c и столкновение глобальных имён из главы 8
$ bash code/ch09/c/run.sh                 # ops.c: таблица указателей на функции из главы 9
$ bash code/ch10/c/run.sh                 # sort.c: qsort против макроса из главы 10
$ bash code/ch11/c/run.sh                 # fops.c: таблица, предсказатель и retpoline из главы 11
$ bash code/ch12/c/run.sh                 # callback.c и nested.c: void *ctx и трамплин из главы 12
$ bash code/ch13/c/run.sh                 # loops.c: ошибки на единицу и на знак из главы 13
$ bash code/ch14/c/run.sh                 # kref.c и twice.c: счётчик ссылок вручную из главы 14
$ bash code/ch15/c/run.sh                 # table.c: самодельная хеш-таблица и висячий указатель из главы 15
$ bash code/ch23/c/run.sh                 # macros.c: SQUARE, MAX, SWAP, LOG2 и X-macro из главы 23
$ bash code/ch25/c/run.sh                 # const int, constexpr C23 и делитель UART в #if из главы 25
$ bash code/ch26/c/run.sh                 # snprintf из newlib и memset, зовущая саму себя, из главы 26
$ bash code/ch18/c/run.sh                 # C вызывает Rust: непрозрачный счётчик (раздел 18.4) под ASan
$ bash solutions/ch18/c/run.sh            # C управляет кольцевым буфером (задача C) под ASan
$ cargo fmt --all --check --manifest-path code/ch08/ffi/Cargo.toml   # и так каждое вложенное
      # рабочее пространство: code/ch07/c-size, code/ch08/{typo,unify,ffi},
      # {exercises,solutions}/ch08/c/station, {exercises,solutions}/ch08/d-station,
      # code/ch24/renamed, code/ch26/start, {exercises,solutions}/ch26/tiny,
      # {code,exercises,solutions}/ch31/fuzz, code/ch31/supply/{build/app,vet}
$ bash solutions/ch01/c/run.sh            # задача 1.A на C
$ CH31_REQUIRE_SANITIZERS=1 cargo test -p ch31 --test sanitizers   # ASan и TSan над san (раздел 31.5)
$ bash code/ch31/fuzz/run.sh              # исходы фаззинга раздела 31.4
$ bash solutions/ch31/fuzz/run.sh         # фаззинг задачи 31.B: внедрённые ошибки найдены, решения чисты
$ bash code/ch31/miri/run.sh              # опыты san под Miri (раздел 31.6)
$ cargo +nightly miri test -p ch31 -p ch31-irq -p ch31-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ RUSTFLAGS="--cfg loom" cargo test -p ch31 --release --test loom --target-dir target/loom
$ bash code/ch31/kani/run.sh              # вердикты Kani разделов 31.7, 31.10 и задач C и D
$ bash code/ch31/c/run.sh                 # разбор команд на C: ASan, libFuzzer, CBMC
$ bash code/ch31/mutants/run.sh           # cargo-mutants на разборе кадров (раздел 31.8)
$ CH31_REQUIRE_MUTANTS=1 cargo test -p ch31-solutions   # задача D: мутанты хранилища убиты
$ bash code/ch31/supply/run.sh            # cargo-deny, cargo-audit, cargo-vet (раздел 31.9)
$ python3 tools/check-includes.py         # все {{#include}} указывают на существующие якоря, ссылки — на страницы
$ python3 tools/check-terms.py            # термины: определены в своём разделе, первое упоминание — ссылка в глоссарий
$ mdbook build
$ python3 tools/check-links.py book       # в собранной книге якоря ссылок существуют, номера разделов в тексте ссылок верны
```

Статья глоссария (`src/appendix-f.md`) — термин, в скобках английский
оригинал, определение, после «→» раздел, где термин введён, и
комментарий `<!-- term: образец -->` (`term-cs` — с учётом регистра, в
обратных кавычках — образец для кода, `once` — ссылка только при первом
упоминании в книге, `only: ЕДИНИЦА...` и `skip: ЕДИНИЦА...` — термин
размечается только в перечисленных единицах или везде, кроме них).
`tools/check-terms.py` требует, чтобы в указанном
разделе термин был выделен курсивом или жирным рядом с оригиналом, а
первое упоминание в каждой главе было определением или ссылкой
`[текст][g-id]`. `--link ЕДИНИЦА...` расставляет такие ссылки, `--refs`
пишет в конец файлов их определения с подсказкой — первой фразой статьи,
`--report` показывает, где термин встречается впервые. CI проверяет
всю книгу.

Задание CI со случайной раскладкой полей прогоняет решения под тремя
зёрнами: эталонный код не должен зависеть от раскладки `repr(Rust)`.
Тесты группы `observed` в `code/ch06` фиксируют поведение текущего rustc и
под случайной раскладкой падают намеренно (раздел 6.2), поэтому там они
пропускаются. В главах 3–5 там же пропускаются тесты, которые запускают
закреплённый rustc или сверяют ассемблер с ответом (Polonius, задачи 3.C,
4.A и 4.B), в главе 7 — задачи 7.A и 7.C, в главе 8 — опыты с cargo и
задачи, которые запускают rustc, cargo или `make`, в главе 9 — снимок
`derive` и пробы задач 9.A, 9.B и 9.C, в главах 10–13 — замеры кода и задачи,
которые собирают пробы или крейт закреплённым тулчейном, в главе 23 —
снимки раскрытия, ассемблер и тексты `stringify!`: к раскладке они не
относятся, а в этом задании установлен только nightly. Их проверяет
основное задание.
В главе 24 так же пропускаются снимки trybuild и раскрытия и тесты,
которые сами запускают cargo (`rustc_*`), и группа `observed`: пробелы
в выводе `stringify!` зависят от версии компилятора, а порядок полей
`repr(Rust)` под случайной раскладкой меняется намеренно, как в главе 6.
В главе 25 — тесты `rustc_*` (ассемблер, секции объектных файлов, пробы и
сборка с урезанным бюджетом) и группа `observed`: общие адреса копий
константы и размеры `LazyLock` и `Mutex` — свойства текущего rustc и `std`,
а не языка. В главе 26 — тесты `rustc_*`: программы без libc, ассемблер
вызовов `memcpy` и тексты ошибок закреплённого компилятора.

## Лицензия

Код примеров, заготовок и решений объявлен в `Cargo.toml` под
MIT OR Apache-2.0. Лицензия текста книги пока не выбрана.
