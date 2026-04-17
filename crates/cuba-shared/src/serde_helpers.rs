//! Serde 辅助函数
//!
//! 提供常用的自定义序列化/反序列化函数

pub mod date_format {
    //! Date 类型的自定义反序列化，支持 YYYY-MM-DD 格式
    use serde::{self, Deserialize, Deserializer};
    use time::Date;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Date::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT)
            .or_else(|_| {
                // 尝试简单的 YYYY-MM-DD 格式
                let format = time::macros::format_description!("[year]-[month]-[day]");
                Date::parse(&s, &format)
            })
            .map_err(serde::de::Error::custom)
    }
}

pub mod optional_date_format {
    //! Option<Date> 类型的自定义反序列化，支持 YYYY-MM-DD 格式
    use serde::{self, Deserialize, Deserializer};
    use time::Date;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            None => Ok(None),
            Some(s) => {
                let date = Date::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT)
                    .or_else(|_| {
                        let format = time::macros::format_description!("[year]-[month]-[day]");
                        Date::parse(&s, &format)
                    })
                    .map_err(serde::de::Error::custom)?;
                Ok(Some(date))
            }
        }
    }
}
