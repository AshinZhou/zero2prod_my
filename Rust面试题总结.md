# Rust 面试题总结

> 🎯 **适用对象**：准备 Rust 后端开发岗位面试的开发者  
> 📅 **更新时间**：2025年1月  
> 💡 **包含内容**：基础概念 + 进阶话题 + 实践问题

---

## 📋 目录

1. [基础语法与概念](#基础语法与概念)
2. [所有权系统](#所有权系统)  
3. [生命周期与借用检查](#生命周期与借用检查)
4. [错误处理](#错误处理)
5. [异步编程](#异步编程)
6. [trait 与泛型](#trait-与泛型)
7. [内存管理](#内存管理)
8. [并发与线程安全](#并发与线程安全)
9. [Web 开发相关](#web-开发相关)
10. [性能优化](#性能优化)
11. [实际项目问题](#实际项目问题)

---

## 🚀 基础语法与概念

### Q1: Rust 的主要设计目标是什么？
**答案要点：**
- **内存安全**：零成本抽象，无运行时垃圾回收
- **线程安全**：防止数据竞争，无恐并发
- **高性能**：零成本抽象，接近 C/C++ 性能
- **跨平台**：支持多种目标架构

### Q2: 解释 Rust 中的 `Option<T>` 和 `Result<T, E>` 区别
**参考答案：**
```rust
// Option<T> - 表示可能有值也可能没有值
let maybe_number: Option<i32> = Some(42);
let no_number: Option<i32> = None;

// Result<T, E> - 表示可能成功也可能失败
let success: Result<i32, String> = Ok(42);
let failure: Result<i32, String> = Err("出错了".to_string());

// 使用场景区别
fn find_user(id: u32) -> Option<User> { /* 用户可能不存在 */ }
fn parse_config() -> Result<Config, ConfigError> { /* 解析可能失败 */ }
```

### Q3: `let` 绑定默认是可变还是不可变的？
**答案：** 默认不可变（immutable）
```rust
let x = 5;        // 不可变
// x = 6;         // 编译错误！

let mut y = 5;    // 可变
y = 6;            // 正确
```

---

## 🏠 所有权系统

### Q4: 解释 Rust 的三个所有权规则
**答案：**
1. **每个值都有一个所有者**
2. **同时只能有一个所有者**  
3. **当所有者离开作用域时，值被丢弃**

### Q5: 下面代码会发生什么？为什么？
```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;
    println!("{}", s1);  // 这行会怎么样？
}
```
**答案：** 编译错误。`s1` 的所有权移动到了 `s2`，所以 `s1` 不再有效。

**正确方式：**
```rust
// 方式1：克隆
let s2 = s1.clone();

// 方式2：借用
let s2 = &s1;
```

### Q6: 什么时候会发生所有权转移（move）？
**答案：**
- 将变量赋值给另一个变量
- 将变量传递给函数
- 从函数返回变量
- 只有实现了 `Copy` trait 的类型例外（如基本数据类型）

---

## ⏰ 生命周期与借用检查

### Q7: 解释这段代码的生命周期问题
```rust
fn longest(x: &str, y: &str) -> &str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```
**答案：** 编译错误，返回的引用生命周期不明确。

**正确写法：**
```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

### Q8: 什么是悬垂引用（Dangling Reference）？
**答案：** 指向已经被释放内存的引用。Rust 编译器会阻止这种情况：
```rust
fn dangle() -> &String {  // 编译错误！
    let s = String::from("hello");
    &s  // s 在函数结束时被销毁，返回引用变成悬垂引用
}

// 正确方式
fn no_dangle() -> String {
    let s = String::from("hello");
    s  // 转移所有权
}
```

---

## ❌ 错误处理

### Q9: `panic!` 和 `Result` 什么时候使用？
**答案：**
- **`panic!`**: 不可恢复的错误，程序应该终止
- **`Result`**: 可恢复的错误，调用者可以处理

```rust
// 使用 panic! - 程序逻辑错误
fn divide(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        panic!("除零错误！");  // 这是程序错误
    }
    a / b
}

// 使用 Result - 外部输入错误
fn parse_number(s: &str) -> Result<i32, std::num::ParseIntError> {
    s.parse()  // 用户输入可能无效
}
```

### Q10: 解释 `?` 运算符的工作原理
**答案：** `?` 是错误传播的语法糖：
```rust
// 使用 ? 运算符
fn read_file() -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string("file.txt")?;
    Ok(content.to_uppercase())
}

// 等价于
fn read_file_explicit() -> Result<String, std::io::Error> {
    let content = match std::fs::read_to_string("file.txt") {
        Ok(content) => content,
        Err(e) => return Err(e),
    };
    Ok(content.to_uppercase())
}
```

---

## ⚡ 异步编程

### Q11: `async/await` 的工作原理是什么？
**答案：**
- `async fn` 返回一个 `Future`
- `await` 暂停当前执行，等待 `Future` 完成
- 需要执行器（如 Tokio）来运行异步任务

```rust
async fn fetch_data() -> Result<String, reqwest::Error> {
    let response = reqwest::get("https://api.example.com/data").await?;
    let text = response.text().await?;
    Ok(text)
}

#[tokio::main]
async fn main() {
    match fetch_data().await {
        Ok(data) => println!("获取到数据: {}", data),
        Err(e) => println!("错误: {}", e),
    }
}
```

### Q12: 什么是 `Send` 和 `Sync` trait？
**答案：**
- **`Send`**: 可以在线程间转移所有权
- **`Sync`**: 可以在多线程间共享引用

```rust
// Arc<T> 是 Send + Sync，可以在线程间共享
use std::sync::Arc;

let data = Arc::new(vec![1, 2, 3]);
let data_clone = Arc::clone(&data);

std::thread::spawn(move || {
    println!("{:?}", data_clone);  // 在另一个线程中使用
});
```

---

## 🧬 Trait 与泛型

### Q13: trait 和接口有什么区别？
**答案：**
```rust
// Trait 可以提供默认实现
trait Drawable {
    fn draw(&self);
    
    // 默认实现
    fn area(&self) -> f64 {
        0.0
    }
}

// 可以为现有类型实现 trait（孤儿规则限制）
impl Drawable for i32 {
    fn draw(&self) {
        println!("绘制数字: {}", self);
    }
}
```

### Q14: 解释泛型的单态化（Monomorphization）
**答案：** 编译器为每个具体类型生成专门的代码版本：
```rust
fn print_value<T: std::fmt::Display>(value: T) {
    println!("{}", value);
}

// 编译器生成：
// fn print_value_i32(value: i32) { println!("{}", value); }
// fn print_value_string(value: String) { println!("{}", value); }
```

---

## 🧠 内存管理

### Q15: Rust 如何防止内存泄漏？
**答案：**
- **RAII**：资源获取即初始化
- **Drop trait**：自动析构
- **所有权系统**：确保资源清理

```rust
struct FileHandle {
    name: String,
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        println!("关闭文件: {}", self.name);
    }
}  // 离开作用域时自动调用 drop
```

### Q16: 什么时候使用 `Box`、`Rc`、`Arc`？
**答案：**
```rust
// Box<T> - 堆分配，单一所有者
let boxed = Box::new(42);

// Rc<T> - 引用计数，单线程共享
use std::rc::Rc;
let rc_data = Rc::new(42);
let rc_clone = Rc::clone(&rc_data);

// Arc<T> - 原子引用计数，多线程共享
use std::sync::Arc;
let arc_data = Arc::new(42);
let arc_clone = Arc::clone(&arc_data);
```

---

## 🔀 并发与线程安全

### Q17: 如何在 Rust 中安全地共享可变数据？
**答案：**
```rust
use std::sync::{Arc, Mutex};
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}
```

### Q18: Channel 的使用场景是什么？
**答案：** 线程间通信，实现生产者-消费者模式：
```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    tx.send("Hello from thread").unwrap();
});

let received = rx.recv().unwrap();
println!("收到消息: {}", received);
```

---

## 🌐 Web 开发相关

### Q19: 在 Actix-Web 中如何处理中间件？
**答案：**
```rust
use actix_web::{web, App, HttpServer, HttpResponse, Result};
use actix_web::middleware::Logger;

async fn hello() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json("Hello World"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())  // 添加日志中间件
            .route("/hello", web::get().to(hello))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Q20: 如何处理数据库连接池？
**答案：**
```rust
use sqlx::{PgPool, PgPoolOptions};

async fn create_pool() -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .connect("postgresql://user:pass@localhost/db")
        .await
}

async fn get_user(pool: &PgPool, id: i32) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT id, name, email FROM users WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
}
```

---

## ⚡ 性能优化

### Q21: 如何避免不必要的内存分配？
**答案：**
```rust
// 避免不必要的 String 分配
fn process_string(s: &str) -> &str {  // 使用 &str 而不是 String
    s.trim()
}

// 使用 Cow 延迟分配
use std::borrow::Cow;

fn maybe_uppercase(s: &str) -> Cow<str> {
    if s.contains(' ') {
        Cow::Owned(s.to_uppercase())  // 只在需要时分配
    } else {
        Cow::Borrowed(s)  // 直接借用
    }
}
```

### Q22: 什么时候使用 `Vec` 预分配容量？
**答案：**
```rust
// 预分配可以减少重新分配
let mut vec = Vec::with_capacity(1000);  // 预分配1000个元素的空间

// 对比：
let mut vec = Vec::new();  // 可能需要多次重新分配
for i in 0..1000 {
    vec.push(i);
}
```

---

## 💼 实际项目问题

### Q23: 如何设计一个错误处理策略？
**答案：**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误")]
    Database(#[from] sqlx::Error),
    
    #[error("网络请求错误")]
    Http(#[from] reqwest::Error),
    
    #[error("配置错误: {message}")]
    Config { message: String },
    
    #[error("验证失败: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

### Q24: 如何实现优雅关闭？
**答案：**
```rust
use tokio::signal;
use tokio::sync::oneshot;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("无法监听 Ctrl+C 信号");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("无法监听 terminate 信号")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    println!("收到关闭信号，开始优雅关闭...");
}
```

### Q25: 如何实现请求限流？
**答案：**
```rust
use std::sync::Arc;
use tokio::sync::Semaphore;
use actix_web::{web, HttpResponse, Result};

pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(max_requests: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_requests)),
        }
    }
    
    pub async fn acquire(&self) -> Option<tokio::sync::SemaphorePermit> {
        self.semaphore.try_acquire().ok()
    }
}

async fn limited_endpoint(
    limiter: web::Data<RateLimiter>
) -> Result<HttpResponse> {
    match limiter.acquire().await {
        Some(_permit) => {
            // 处理请求
            Ok(HttpResponse::Ok().json("请求成功"))
        }
        None => {
            Ok(HttpResponse::TooManyRequests().json("请求过于频繁"))
        }
    }
}
```

---

## 📝 面试准备建议

### 💡 **技术深度**
1. **掌握所有权系统**：这是 Rust 最核心的特性
2. **理解异步编程**：现代 Rust 后端开发必备
3. **熟悉错误处理**：优雅的错误处理是专业代码的标志
4. **了解性能优化**：零成本抽象的具体应用

### 🎯 **实践经验**
1. **完整项目经历**：能够描述从设计到部署的完整流程
2. **遇到的困难**：如何解决编译器错误、性能问题等
3. **技术选型**：为什么选择某个库或架构模式
4. **代码质量**：测试、文档、代码审查经验

### 📚 **持续学习**
1. **关注 Rust 生态**：新的 crate 和最佳实践
2. **参与开源项目**：贡献代码或参与讨论
3. **技术博客**：分享学习心得和项目经验
4. **社区参与**：Rust 用户组、会议、论坛

---

## 🔗 相关资源

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust By Example](https://doc.rust-lang.org/rust-by-example/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Actix-Web Documentation](https://actix.rs/)
- [SQLx Documentation](https://docs.rs/sqlx/)

---

> 💡 **面试贴士**：准备具体的代码示例，能够现场编写简单的 Rust 程序，展示对语言特性的深入理解。 