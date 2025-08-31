use crate::letters::LetterIdIndexed;

pub const HAJIMI: [&str; 10] = [
    "哈基米",
    "那呗路多",
    "阿西嘎",
    "嗨呀库",
    "欧嘛滋哩",
    "曼波",
    "大狗叫",
    "叮咚鸡",
    "呜哦",
    "哇恰",
];

pub fn hajimi_tokens() -> LetterIdIndexed<String> {
    LetterIdIndexed::new(HAJIMI.iter().map(|s| s.to_string()).collect())
}
