//! Одно приложение на шине и два запроса сразу: ждёт ли второй, пока отвечают
//! первому.
//!
//! Повод вполне конкретный. Приложение, которое держит запрос (длинный опрос —
//! «подожди, пока появится что отдать»), на шине обрекает на ожидание всех: его
//! функция обслуживается по одному запросу за раз, а остальные тем временем
//! лежат в арене запросов. Арена кончается, и зовущие получают `function
//! request ring full` — то есть отказ не от приложения, а от переполненного
//! кольца, при пустом журнале у приложения: эти запросы оно ещё не читало.
//!
//! Тест ставит три пути — медленный (держит полторы секунды), быстрый и
//! падающий — и смотрит, когда вернётся быстрый, посланный сразу за медленным,
//! и слышит ли приложение что-нибудь после паники в обработчике.
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use impulse_client_ring::ImpulseRingClient;
use impulse_server_kit::prelude::*;
use impulse_server_kit::setup::ProtocolConfig;
use serde::Deserialize;

const APP: &str = "concurrency-e2e";

/// Сколько держит медленный обработчик. Больше дороги по шине на столько, чтобы
/// разница между «ждал» и «не ждал» не умещалась в погрешность.
const SLOW: Duration = Duration::from_millis(1500);

struct Broker(Child);
impl Drop for Broker {
  fn drop(&mut self) {
    let _ = self.0.kill();
    let _ = self.0.wait();
  }
}

fn broker_binary() -> Option<PathBuf> {
  if let Ok(p) = std::env::var("IMPULSED_BIN") {
    let p = PathBuf::from(p);
    if p.exists() {
      return Some(p);
    }
  }
  let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  for rel in [
    "../../impulse-ring/target/debug/impulsed",
    "../impulse-ring/target/debug/impulsed",
  ] {
    let cand = manifest.join(rel);
    if cand.exists() {
      return Some(cand);
    }
  }
  None
}

fn cleanup_shm() {
  if let Ok(entries) = std::fs::read_dir("/dev/shm") {
    for entry in entries.flatten() {
      if entry.file_name().to_string_lossy().starts_with("impulse-ring.") {
        let _ = std::fs::remove_file(entry.path());
      }
    }
  }
}

fn start_broker() -> Option<Broker> {
  let bin = broker_binary()?;
  cleanup_shm();
  let child = Command::new(bin).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()?;
  Some(Broker(child))
}

#[derive(Deserialize, Default, Clone)]
struct Setup {
  #[serde(flatten)]
  generic_values: GenericValues,
}

impl GenericSetup for Setup {
  fn generic_values(&self) -> &GenericValues {
    &self.generic_values
  }
  fn generic_values_mut(&mut self) -> &mut GenericValues {
    &mut self.generic_values
  }
}

/// Держит запрос, как длинный опрос: ничего не считает, просто не отвечает.
#[handler]
async fn slow() -> &'static str {
  tokio::time::sleep(SLOW).await;
  "slow"
}

#[handler]
async fn fast() -> &'static str {
  "fast"
}

/// Паникует. Обработчик, который так делает, не должен уносить с собой
/// обслуживание приложения: ответ на этот запрос — ошибка, а на следующий —
/// обычный ответ.
#[handler]
async fn boom() -> &'static str {
  panic!("обработчик упал");
}

async fn connect_with_retry(app: &str) -> ImpulseRingClient {
  for _ in 0..100 {
    if let Ok(c) = ImpulseRingClient::connect(app) {
      return c;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  panic!("не удалось подключиться к шине");
}

/// Один брокер и одно приложение на весь файл: контрольный сегмент на машине
/// один, поэтому два теста с собственными брокерами сносили бы друг друга (и
/// сносили: один висел, другой падал). Проверки идут подряд, как в соседних
/// сквозных тестах этого крейта.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_held_request_does_not_hold_the_rest_and_a_panic_does_not_deafen() {
  let Some(_broker) = start_broker() else {
    eprintln!("пропуск: не найден брокер `impulsed` (задайте IMPULSED_BIN)");
    return;
  };
  tokio::time::sleep(Duration::from_millis(300)).await;

  let mut setup = Setup::default();
  setup.generic_values.app_name = APP.to_string();
  setup.generic_values.protocols = vec![ProtocolConfig::ImpulseRing {
    app_name: APP.to_string(),
    access_key: None,
    arena_size_kib: None,
  }];
  let state = load_generic_state(&setup, false).await.unwrap();
  let router = get_root_router_autoinject(&state, setup.clone())
    .push(Router::with_path("slow").get(slow))
    .push(Router::with_path("fast").get(fast))
    .push(Router::with_path("boom").get(boom));
  let (server, _handle) = start(state, &setup, router).await.unwrap();
  let _server_task = tokio::spawn(server);

  let client = connect_with_retry(APP).await;
  // Подключение доходит только до брокера; функция приложения могла ещё не
  // выставиться.
  for _ in 0..100 {
    if client.get("/fast").send().await.is_ok() {
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }

  // --- Удержанный запрос не держит остальные.
  //
  // Медленный ушёл первым, другим соединением (как другой зовущий), быстрый —
  // сразу за ним.
  let slow_client = connect_with_retry(APP).await;
  let began = Instant::now();
  let slow_call = tokio::spawn(async move { slow_client.get("/slow").send().await.map(|r| r.status().as_u16()) });
  tokio::time::sleep(Duration::from_millis(100)).await;

  let resp = client.get("/fast").send().await.expect("быстрый запрос не прошёл");
  assert_eq!(resp.status().as_u16(), 200);
  let fast_at = began.elapsed();
  assert_eq!(slow_call.await.unwrap().unwrap(), 200);
  assert!(
    fast_at < SLOW,
    "быстрый запрос вернулся через {fast_at:?}, то есть дождался медленного ({SLOW:?}): \
     приложение на шине обслуживает по одному запросу за раз, и любой длинный опрос \
     останавливает всё остальное"
  );

  // --- Упавший обработчик отвечает ошибкой — и приложение по-прежнему слышит.
  //
  // Иначе одна паника делает его глухим навсегда: на шине оно выставлено,
  // брокер его видит, а запросы копятся в арене, пока зовущие не начнут
  // получать `function request ring full`.
  let _ = client.get("/boom").send().await;
  let resp = tokio::time::timeout(Duration::from_secs(5), client.get("/fast").send())
    .await
    .expect("после паники приложение перестало отвечать")
    .expect("после паники запрос не прошёл");
  assert_eq!(resp.status().as_u16(), 200);
}
