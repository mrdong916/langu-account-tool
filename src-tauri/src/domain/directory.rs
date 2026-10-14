use super::workspace::Account;
use pinyin::ToPinyin;
use serde::Serialize;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryStatus {
    pub scope: String,
    pub syncing: bool,
    pub count: usize,
    pub fetched: usize,
    pub total: u64,
    pub last_sync: Option<i64>,
    pub last_full_sync: Option<i64>,
    pub error: String,
}

pub fn normalize(text: &str) -> String {
    text.to_lowercase().split_whitespace().collect::<String>()
}

pub fn index(account: &mut Account) {
    let text = format!(
        "{} {} {} {}",
        account.username, account.realname, account.work_no, account.department
    );
    let mut full = String::new();
    let mut initials = String::new();
    for (character, syllable) in text.chars().zip(text.as_str().to_pinyin()) {
        if let Some(syllable) = syllable {
            full.push_str(syllable.plain());
            initials.push(syllable.first_letter().chars().next().unwrap());
        } else {
            full.push(character);
            initials.push(character);
        }
    }
    // Common polyphonic surnames; retain the ordinary reading as an alias.
    for (surname, full_name, initial) in [
        ('单', "shan", "s"),
        ('解', "xie", "x"),
        ('仇', "qiu", "q"),
        ('曾', "zeng", "z"),
        ('区', "ou", "o"),
    ] {
        if account.realname.starts_with(surname) {
            let tail: String = account.realname.chars().skip(1).collect();
            let tail_full: String = tail
                .as_str()
                .to_pinyin()
                .flatten()
                .map(|p| p.plain())
                .collect();
            let tail_initials: String = tail
                .as_str()
                .to_pinyin()
                .flatten()
                .map(|p| p.first_letter())
                .collect();
            full.push_str(&format!(" {full_name}{tail_full} {initial}{tail_initials}"));
        }
    }
    account.search_text = normalize(&format!("{text} {full} {initials}"));
}

pub fn score(account: &Account, query: &str) -> Option<u8> {
    let terms: Vec<_> = query.split_whitespace().map(normalize).collect();
    if !terms.iter().all(|term| account.search_text.contains(term)) {
        return None;
    }
    let query = normalize(query);
    let names = [&account.username, &account.work_no, &account.realname];
    Some(if names.iter().any(|name| normalize(name) == query) {
        0
    } else if names.iter().any(|name| normalize(name).starts_with(&query)) {
        1
    } else {
        2
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn searches_identity_department_and_pinyin() {
        let mut user = Account {
            username: "A1000".into(),
            realname: "张三".into(),
            department: "财务部".into(),
            ..Default::default()
        };
        index(&mut user);
        for query in ["张三", "zhangsan", "zs", "A1000", "财务 张", "CW ZS"] {
            assert!(score(&user, query).is_some(), "{query}");
        }
        assert_eq!(score(&user, "a1000"), Some(0));
        assert_eq!(score(&user, "李四"), None);
        user.realname = "单明".into();
        index(&mut user);
        assert!(score(&user, "shanming").is_some());
    }
}
