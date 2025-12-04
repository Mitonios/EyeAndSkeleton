use crate::config::AppConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

/// Events sent from timer to main thread
#[derive(Debug)]
pub enum TimerEvent {
    ShowBlink,
    ShowStandUp,
}

/// Manager cho 2 timers độc lập
pub struct TimerManager {
    config: Arc<AppConfig>,
    blink_interval: time::Interval,
    standup_interval: time::Interval,
    next_standup_time: time::Instant,
}

impl TimerManager {
    /// Tạo mới TimerManager
    pub fn new(config: Arc<AppConfig>) -> Self {
        let now = time::Instant::now();

        // Tạo blink interval
        let blink_duration = Duration::from_secs(config.blink_interval as u64 * 60);
        let mut blink_interval = time::interval_at(now + blink_duration, blink_duration);
        blink_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        // Tạo standup interval
        let standup_duration = Duration::from_secs(config.standup_interval as u64 * 60);
        let mut standup_interval = time::interval_at(now + standup_duration, standup_duration);
        standup_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        let next_standup_time = now + standup_duration;

        Self {
            config,
            blink_interval,
            standup_interval,
            next_standup_time,
        }
    }

    /// Cập nhật config và restart timers
    pub fn update_config(&mut self, config: Arc<AppConfig>) {
        self.config = config.clone();
        let now = time::Instant::now();

        // Restart blink timer
        let blink_duration = Duration::from_secs(self.config.blink_interval as u64 * 60);
        self.blink_interval = time::interval_at(now + blink_duration, blink_duration);
        self.blink_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        // Restart standup timer
        let standup_duration = Duration::from_secs(self.config.standup_interval as u64 * 60);
        self.standup_interval = time::interval_at(now + standup_duration, standup_duration);
        self.standup_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        self.next_standup_time = now + standup_duration;

        log::info!("Đã cập nhật timers với config mới: blink={}min, standup={}min",
                  self.config.blink_interval, self.config.standup_interval);
    }

    /// Chạy timers với khả năng update config
    pub async fn run_with_updates(
        mut self,
        tx: mpsc::Sender<TimerEvent>,
        mut update_rx: mpsc::Receiver<Arc<AppConfig>>,
    ) -> anyhow::Result<()> {
        log::info!("Bắt đầu chạy timers với update support: blink={}min, standup={}min",
                  self.config.blink_interval, self.config.standup_interval);

        loop {
            tokio::select! {
                // Config update
                Some(new_config) = update_rx.recv() => {
                    log::info!("Nhận config update: blink={}min, standup={}min",
                              new_config.blink_interval, new_config.standup_interval);
                    self.update_config(new_config);
                }

                // Blink timer fired
                _ = self.blink_interval.tick() => {
                    // Kiểm tra xem standup có trùng không
                    let now = time::Instant::now();
                    let time_to_standup = self.next_standup_time.saturating_duration_since(now);

                    // Nếu standup sẽ hiển thị trong vòng 10 giây tới, bỏ qua blink
                    if time_to_standup <= Duration::from_secs(10) {
                        log::info!("Bỏ qua blink vì standup sắp hiển thị ({}s)", time_to_standup.as_secs());
                        continue;
                    }

                    log::info!("Timer blink kích hoạt");
                    if tx.send(TimerEvent::ShowBlink).await.is_err() {
                        log::warn!("Không thể gửi TimerEvent::ShowBlink");
                        break;
                    }
                }

                // Standup timer fired
                _ = self.standup_interval.tick() => {
                    log::info!("Timer standup kích hoạt");

                    // Cập nhật next standup time
                    self.next_standup_time = time::Instant::now() +
                        Duration::from_secs(self.config.standup_interval as u64 * 60);

                    if tx.send(TimerEvent::ShowStandUp).await.is_err() {
                        log::warn!("Không thể gửi TimerEvent::ShowStandUp");
                        break;
                    }
                }
            }
        }

        log::info!("Timer manager dừng hoạt động");
        Ok(())
    }
}

/// Handle cho việc update config của timer manager
#[derive(Clone)]
pub struct TimerHandle {
    update_tx: mpsc::Sender<Arc<AppConfig>>,
}

impl TimerHandle {
    /// Cập nhật config cho timer manager
    pub async fn update_config(&self, config: Arc<AppConfig>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.update_tx.send(config).await?;
        Ok(())
    }
}

/// Khởi động timers trong background task và trả về handle để update config
pub fn start_timers(
    config: Arc<AppConfig>,
    tx: mpsc::Sender<TimerEvent>,
) -> anyhow::Result<TimerHandle> {
    let timer_manager = TimerManager::new(config);
    let (update_tx, update_rx) = mpsc::channel(32);

    tokio::spawn(async move {
        if let Err(e) = timer_manager.run_with_updates(tx, update_rx).await {
            log::error!("Timer manager gặp lỗi: {}", e);
        }
    });

    Ok(TimerHandle { update_tx })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_timer_creation() {
        let config = Arc::new(AppConfig {
            startup: false,
            blink_interval: 1,
            standup_interval: 2,
        });

        let manager = TimerManager::new(config);
        assert_eq!(manager.config.blink_interval, 1);
        assert_eq!(manager.config.standup_interval, 2);
    }

    #[tokio::test]
    async fn test_config_update() {
        let config1 = Arc::new(AppConfig {
            startup: false,
            blink_interval: 1,
            standup_interval: 2,
        });

        let mut manager = TimerManager::new(config1);

        let config2 = Arc::new(AppConfig {
            startup: true,
            blink_interval: 5,
            standup_interval: 30,
        });

        manager.update_config(config2);
        assert_eq!(manager.config.blink_interval, 5);
        assert_eq!(manager.config.standup_interval, 30);
    }
}
