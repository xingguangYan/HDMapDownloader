//! 中英双语切换（v4.0）
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Lang {
    Zh,
    En,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::Zh
    }
}

/// 传入中/英两份文案，按当前语言返回
pub fn t<'a>(lang: &Lang, zh: &'a str, en: &'a str) -> &'a str {
    if matches!(*lang, Lang::En) {
        en
    } else {
        zh
    }
}