use crate::models::{account::AccountData, network::NetworkRequest, proxy::ProxyInfo};
use ratatui::widgets::ListState;

#[derive(Debug, PartialEq, Clone)]
pub enum AppMode {
    Normal,
    SelectCookiesMethod,
    SelectProxiesMethod,
    InputPath,
    PasteText,
    InputProxyPath,
    PasteProxyText,
    Scanning,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AppTab {
    System,
    Network,
}

#[derive(Debug, PartialEq, Clone)]
pub enum InnerNetworkTab {
    Headers,
    Body,
    Cookies,
}

pub struct App {
    pub mode: AppMode,
    pub active_tab: AppTab,
    pub network_inner_tab: InnerNetworkTab,
    pub accounts: Vec<AccountData>,
    pub selected_account: usize,
    pub should_quit: bool,
    pub input_buffer: String,
    pub logs: Vec<String>,
    pub tick: u64,
    pub proxies: Vec<ProxyInfo>,
    pub log_scroll: usize,
    pub network_requests: Vec<NetworkRequest>,
    pub selected_network_request: usize,
    pub network_scroll: usize,
    pub account_state: ListState,
    pub network_state: ListState,
    pub logs_state: ListState,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Normal,
            active_tab: AppTab::System,
            network_inner_tab: InnerNetworkTab::Body,
            accounts: Vec::new(),
            selected_account: 0,
            should_quit: false,
            input_buffer: String::new(),
            logs: Vec::new(),
            tick: 0,
            proxies: Vec::new(),
            log_scroll: 0,
            network_requests: Vec::new(),
            selected_network_request: 0,
            network_scroll: 0,
            account_state: ListState::default(),
            network_state: ListState::default(),
            logs_state: ListState::default(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn add_log(&mut self, msg: String) {
        self.logs.push(msg);
        self.log_scroll = 0;
        if self.logs.len() > 10000 {
            self.logs.remove(0);
        }
        self.logs_state
            .select(Some(self.logs.len().saturating_sub(1)));
    }

    pub fn next_account(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_account = (self.selected_account + 1) % self.accounts.len();
            self.account_state.select(Some(self.selected_account));
        }
    }

    pub fn prev_account(&mut self) {
        if !self.accounts.is_empty() {
            if self.selected_account > 0 {
                self.selected_account -= 1;
            } else {
                self.selected_account = self.accounts.len() - 1;
            }
            self.account_state.select(Some(self.selected_account));
        }
    }
}
