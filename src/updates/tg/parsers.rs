use super::TelegramUpdate;
use crate::result::{Error, Result};
use tg_collector::tg_client::TgUpdate;
use tg_collector::{
    FormattedText, Message, MessageContent, MessageText, TextEntity, TextEntityType,
};

pub async fn parse_update(tg_update: &TgUpdate) -> Result<Option<TelegramUpdate>> {
    Ok(match tg_update {
        TgUpdate::NewMessage(new_message) => {
            let content = parse_message_content(new_message.message().content()).await?;
            match content {
                None => None,
                Some(content) => Some(TelegramUpdate {
                    message_id: new_message.message().id(),
                    chat_id: new_message.message().chat_id(),
                    date: Some(new_message.message().date()),
                    content,
                }),
            }
        }
        TgUpdate::MessageContent(message_content) => {
            let content = parse_message_content(message_content.new_content()).await?;
            Some(TelegramUpdate {
                message_id: message_content.message_id(),
                chat_id: message_content.chat_id(),
                date: None,
                content: content.unwrap_or_default(),
            })
        }
        TgUpdate::ChatPhoto(_) => return Err(Error::UpdateNotSupported),
        TgUpdate::ChatTitle(_) => return Err(Error::UpdateNotSupported),
        TgUpdate::Supergroup(_) => return Err(Error::UpdateNotSupported),
        TgUpdate::SupergroupFullInfo(_) => return Err(Error::UpdateNotSupported),
    })
}

pub async fn parse_message_content(message: &MessageContent) -> Result<Option<String>> {
    match message {
        MessageContent::MessageText(text) => Ok(Some(parse_formatted_text(text.text()))),
        MessageContent::MessageAnimation(animation) => {
            Ok(Some(parse_formatted_text(animation.caption())))
        }
        MessageContent::MessageAudio(audio) => Ok(Some(parse_formatted_text(audio.caption()))),
        MessageContent::MessageDocument(document) => {
            Ok(Some(parse_formatted_text(document.caption())))
        }
        MessageContent::MessagePhoto(photo) => Ok(Some(parse_formatted_text(photo.caption()))),
        MessageContent::MessageVideo(video) => Ok(Some(parse_formatted_text(video.caption()))),

        MessageContent::MessageChatChangePhoto(_) => Err(Error::UpdateNotSupported),

        MessageContent::MessagePoll(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageChatChangeTitle(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageChatDeletePhoto(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageChatJoinByLink(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageChatUpgradeFrom(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageChatUpgradeTo(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageContact(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageContactRegistered(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageCustomServiceAction(_) => Err(Error::UpdateNotSupported),
        // MessageContent::MessageDice(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageExpiredPhoto(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageExpiredVideo(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageInvoice(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageLocation(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessagePassportDataReceived(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageScreenshotTaken(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageSticker(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageSupergroupChatCreate(_) => Err(Error::UpdateNotSupported),

        MessageContent::MessageVenue(_) => Err(Error::UpdateNotSupported),

        MessageContent::MessageVideoNote(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageVoiceNote(_) => Err(Error::UpdateNotSupported),
        MessageContent::MessageWebsiteConnected(_) => Err(Error::UpdateNotSupported),

        MessageContent::_Default(_) => Ok(None),
        MessageContent::MessageBasicGroupChatCreate(_) => Ok(None),
        MessageContent::MessageCall(_) => Ok(None),
        MessageContent::MessageChatAddMembers(_) => Ok(None),
        MessageContent::MessageChatDeleteMember(_) => Ok(None),
        MessageContent::MessageChatSetTtl(_) => Ok(None),
        MessageContent::MessageGame(_) => Ok(None),
        MessageContent::MessageGameScore(_) => Ok(None),
        MessageContent::MessagePassportDataSent(_) => Ok(None),
        MessageContent::MessagePaymentSuccessful(_) => Ok(None),
        MessageContent::MessagePaymentSuccessfulBot(_) => Ok(None),
        MessageContent::MessagePinMessage(_) => Ok(None),
        MessageContent::MessageUnsupported(_) => Ok(None),
    }
}

pub fn parse_formatted_text(formatted_text: &FormattedText) -> String {
    let mut entities_by_index = make_entities_stack(formatted_text.entities());
    let mut result_text = String::new();
    let mut current_entity = match entities_by_index.pop() {
        None => return formatted_text.text().clone(),
        Some(entity) => entity,
    };
    for (i, ch) in formatted_text.text().chars().enumerate() {
        if i == current_entity.0 {
            result_text = format!("{}{}{}", result_text, current_entity.1, ch);
            current_entity = match entities_by_index.pop() {
                None => {
                    result_text = format!(
                        "{}{}",
                        result_text,
                        &formatted_text
                            .text()
                            .chars()
                            .skip(i + 1)
                            .take(formatted_text.text().len() - i)
                            .collect::<String>()
                    );
                    return result_text;
                }
                Some(entity) => entity,
            };
        } else {
            result_text.push(ch)
        }
    }
    result_text
}

fn make_entities_stack(entities: &Vec<TextEntity>) -> Vec<(usize, String)> {
    let mut stack = Vec::new();
    for entity in entities {
        let formatting = match entity.type_() {
            TextEntityType::Bold(_) => Some(("<b>".to_string(), "</b>".to_string())),
            TextEntityType::Code(_) => Some(("<code>".to_string(), "</code>".to_string())),
            TextEntityType::Hashtag(_) => Some(("#".to_string(), "".to_string())),
            TextEntityType::Italic(_) => Some(("<i>".to_string(), "</i>".to_string())),
            TextEntityType::PhoneNumber(_) => Some(("<phone>".to_string(), "</phone>".to_string())),
            TextEntityType::Pre(_) => Some(("<pre>".to_string(), "</pre>".to_string())),
            TextEntityType::PreCode(_) => {
                Some(("<pre><code>".to_string(), "</code></pre>".to_string()))
            }
            TextEntityType::Strikethrough(_) => {
                Some(("<strike>".to_string(), "</strike>".to_string()))
            }
            TextEntityType::TextUrl(u) => {
                let tag = format!(r#"<a href="{}">"#, u.url());
                Some((tag, "</a>".to_string()))
            }
            TextEntityType::Underline(_) => Some(("<u>".to_string(), "</u>".to_string())),
            TextEntityType::Url(_) => Some(("<a>".to_string(), "</a>".to_string())),
            TextEntityType::_Default(_) => None,
            // TextEntityType::BankCardNumber(_) => None,
            TextEntityType::BotCommand(_) => None,
            TextEntityType::Cashtag(_) => None,
            TextEntityType::EmailAddress(_) => None,
            TextEntityType::Mention(_) => None,
            TextEntityType::MentionName(_) => None,
        };
        match formatting {
            Some((start_tag, end_tag)) => {
                stack.push((entity.offset().clone() as usize, start_tag));
                stack.push((
                    (entity.offset() + entity.length()).clone() as usize,
                    end_tag,
                ));
            }
            None => {}
        }
    }
    stack.sort_by_key(|(i, _)| *i);
    stack.reverse();
    stack
}

#[cfg(test)]
mod tests {
    use crate::updates::tg::parse_message_text;
    use tg_collector::{FormattedText, MessageText};

    #[tokio::test]
    async fn test_parse_message_text() {
        let tests = vec![
            (
                r#"{"@type":"formattedText","@extra":"","text":"Изображение из пятидесяти линий.\nНаткнулся на скрипт, который генерирует такие изображения вот тут.\nЛожите рядом со скриптом png изображение 750х750 в градациях серого, в исходнике меняете имя файла на ваше и запускаете исходник с помощью processing. Сгенерированное изображение будет лежать в том же каталоге.","entities":[{"@type":"textEntity","@extra":"","offset":91,"length":7,"type":{"@type":"textEntityTypeTextUrl","@extra":"","url":"https://gist.github.com/u-ndefine/8e4bc21be4275f87fefe7b2a68487161"}},{"@type":"textEntity","@extra":"","offset":239,"length":10,"type":{"@type":"textEntityTypeTextUrl","@extra":"","url":"https://processing.org/download/"}}]}"#,
                r#"Изображение из пятидесяти линий.
Наткнулся на скрипт, который генерирует такие изображения <a href="https://gist.github.com/u-ndefine/8e4bc21be4275f87fefe7b2a68487161">вот тут</a>.
Ложите рядом со скриптом png изображение 750х750 в градациях серого, в исходнике меняете имя файла на ваше и запускаете исходник с помощью <a href="https://processing.org/download/">processing</a>. Сгенерированное изображение будет лежать в том же каталоге."#,
            ),
            (
                r#"{"@type":"formattedText","@extra":"","text":"Напоминаем, что здесь у нас есть ещё и свой чат, где проходят «публичные» интервью, а в свободное время можно просто потрещать за жизнь \n\nЗаходи, тебе здесь рады)\n\nhttps://t.me/joinchat/IqlQqUGyZpI1-0Zu8ChAmA","entities":[]}"#,
                r#"Напоминаем, что здесь у нас есть ещё и свой чат, где проходят «публичные» интервью, а в свободное время можно просто потрещать за жизнь 

Заходи, тебе здесь рады)

https://t.me/joinchat/IqlQqUGyZpI1-0Zu8ChAmA"#,
            ),
        ];
        for (json_data, expected) in tests {
            let formatted_text = FormattedText::from_json(json_data).unwrap();
            let message_text = MessageText::builder().text(formatted_text).build();
            let t = parse_message_text(&message_text).await;
            assert_eq!(t, expected);
        }
    }
}
