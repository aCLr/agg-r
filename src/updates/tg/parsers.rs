use super::{TelegramUpdate, TELEGRAM};
use crate::result::{Error, Result};
use crate::updates::tg::{FilePath, FileType, TelegramFile, TelegramFileWithMeta, TelegramMessage};
use tg_collector::tg_client::TgUpdate;
use tg_collector::types::Channel;
use tg_collector::{FormattedText, MessageContent, RObject, TextEntity, TextEntityType};

/// Parses updates from `tg_collector` to `TelegramUpdate` struct
pub async fn parse_update(tg_update: &TgUpdate) -> Result<Option<TelegramUpdate>> {
    Ok(match tg_update {
        TgUpdate::FileDownloaded(update_file) => {
            Some(TelegramUpdate::FileDownloadFinished(TelegramFile {
                local_path: update_file.file().local().path().clone(),
                remote_file: update_file.file().id().to_string(),
                remote_id: update_file.file().remote().unique_id().to_string(),
            }))
        }
        TgUpdate::NewMessage(new_message) => {
            let (content, files) = parse_message_content(new_message.message().content()).await?;
            if content.is_none() && files.is_none() {
                None
            } else {
                Some(TelegramUpdate::Message(TelegramMessage {
                    message_id: new_message.message().id(),
                    chat_id: new_message.message().chat_id(),
                    date: Some(new_message.message().date()),
                    content,
                    files,
                }))
            }
        }
        TgUpdate::MessageContent(message_content) => {
            let (content, files) = parse_message_content(message_content.new_content()).await?;

            if content.is_none() && files.is_none() {
                None
            } else {
                Some(TelegramUpdate::Message(TelegramMessage {
                    message_id: message_content.message_id(),
                    chat_id: message_content.chat_id(),
                    date: None,
                    content,
                    files,
                }))
            }
        }
        TgUpdate::ChatPhoto(photo) => {
            return Err(Error::UpdateNotSupported(photo.td_name().to_string()))
        }
        TgUpdate::ChatTitle(title) => {
            return Err(Error::UpdateNotSupported(title.td_name().to_string()))
        }
    })
}

pub async fn parse_message_content(
    message: &MessageContent,
) -> Result<(Option<String>, Option<Vec<TelegramFileWithMeta>>)> {
    match message {
        MessageContent::MessageText(text) => Ok((Some(parse_formatted_text(text.text())), None)),
        MessageContent::MessageAnimation(message_animation) => {
            let file = TelegramFileWithMeta {
                path: FilePath::new(message_animation.animation().animation()),
                file_type: FileType::Animation(message_animation.animation().into()),
                file_name: Some(message_animation.animation().file_name().clone()),
            };
            Ok((
                Some(parse_formatted_text(message_animation.caption())),
                Some(vec![file]),
            ))
        }
        MessageContent::MessageAudio(audio) => {
            Ok((Some(parse_formatted_text(audio.caption())), None))
        }
        MessageContent::MessageDocument(message_document) => {
            let file = TelegramFileWithMeta {
                path: FilePath::new(message_document.document().document()),
                file_type: FileType::Document,
                file_name: Some(message_document.document().file_name().clone()),
            };
            Ok((
                Some(parse_formatted_text(message_document.caption())),
                Some(vec![file]),
            ))
        }
        MessageContent::MessagePhoto(photo) => {
            let files = photo
                .photo()
                .sizes()
                .iter()
                .map(|s| TelegramFileWithMeta {
                    file_type: FileType::Image(s.into()),
                    path: FilePath::new(s.photo()),
                    file_name: None,
                })
                .collect();
            Ok((Some(parse_formatted_text(photo.caption())), Some(files)))
        }
        MessageContent::MessageVideo(video) => {
            Ok((Some(parse_formatted_text(video.caption())), None))
        }

        MessageContent::MessageChatChangePhoto(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }

        MessageContent::MessagePoll(u) => Err(Error::UpdateNotSupported(u.td_name().to_string())),
        MessageContent::MessageChatChangeTitle(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageChatDeletePhoto(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageChatJoinByLink(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageChatUpgradeFrom(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageChatUpgradeTo(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageContact(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageContactRegistered(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageCustomServiceAction(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageExpiredPhoto(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageExpiredVideo(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageInvoice(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageLocation(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessagePassportDataReceived(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageScreenshotTaken(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageSticker(message_sticker) => {
            let file = TelegramFileWithMeta {
                path: FilePath::new(message_sticker.sticker().sticker()),
                file_type: FileType::Image(message_sticker.sticker().into()),
                file_name: None,
            };
            Ok((None, Some(vec![file])))
        }
        MessageContent::MessageSupergroupChatCreate(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }

        MessageContent::MessageVenue(u) => Err(Error::UpdateNotSupported(u.td_name().to_string())),

        MessageContent::MessageVideoNote(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageVoiceNote(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }
        MessageContent::MessageWebsiteConnected(u) => {
            Err(Error::UpdateNotSupported(u.td_name().to_string()))
        }

        MessageContent::_Default(_) => Ok((None, None)),
        MessageContent::MessageBasicGroupChatCreate(_) => Ok((None, None)),
        MessageContent::MessageCall(_) => Ok((None, None)),
        MessageContent::MessageChatAddMembers(_) => Ok((None, None)),
        MessageContent::MessageChatDeleteMember(_) => Ok((None, None)),
        MessageContent::MessageChatSetTtl(_) => Ok((None, None)),
        MessageContent::MessageGame(_) => Ok((None, None)),
        MessageContent::MessageGameScore(_) => Ok((None, None)),
        MessageContent::MessagePassportDataSent(_) => Ok((None, None)),
        MessageContent::MessagePaymentSuccessful(_) => Ok((None, None)),
        MessageContent::MessagePaymentSuccessfulBot(_) => Ok((None, None)),
        MessageContent::MessagePinMessage(_) => Ok((None, None)),
        MessageContent::MessageUnsupported(_) => Ok((None, None)),
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

fn make_entities_stack(entities: &[TextEntity]) -> Vec<(usize, String)> {
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
        if let Some((start_tag, end_tag)) = formatting {
            stack.push((entity.offset() as usize, start_tag));
            stack.push(((entity.offset() + entity.length()) as usize, end_tag));
        }
    }
    stack.sort_by_key(|(i, _)| *i);
    stack.reverse();
    stack
}

pub(super) fn channel_to_new_source(channel: Channel) -> crate::models::NewSource {
    crate::models::NewSource {
        name: channel.title,
        origin: channel.chat_id.to_string(),
        kind: TELEGRAM.to_string(),
        image: None,
        external_link: channel.username,
    }
}

#[cfg(test)]
mod tests {
    use crate::updates::tg::parsers::parse_formatted_text;
    use tg_collector::FormattedText;

    #[test]
    fn test_parse_formatted_text() {
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
            let t = parse_formatted_text(&formatted_text);
            assert_eq!(t, expected);
        }
    }
}
