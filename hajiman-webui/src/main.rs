use hajiman::{
    Bits, BitsIter, CharacterCounter, CharacterFrequency, JimiDecoder, JimiEncoder, JimiEncoding,
    JimiError, LexemError, bits::Bits8, hajimi_tokens, serde_json,
};
use leptos::prelude::*;

struct JimiState {
    encoding: JimiEncoding<Bits8>,
    decoder: JimiDecoder<Bits8>,
    encoder: JimiEncoder<Bits8>,
}

impl Default for JimiState {
    fn default() -> Self {
        let freq = CharacterFrequency::<Bits8>::all_equal();
        let encoding = JimiEncoding::new(hajimi_tokens(), &freq);
        let (encoder, decoder) = (encoding.encoder(), encoding.decoder().unwrap());

        Self {
            encoding,
            encoder,
            decoder,
        }
    }
}

impl JimiState {
    fn encode_with_current_encoding(&self, input: &str) -> String {
        self.encoder.encode(input.as_bytes()).data.collect()
    }

    fn encode(&mut self, input: &str) -> String {
        if input.is_empty() {
            return "".to_string();
        }

        let freq = CharacterCounter::empty()
            .count(Bits8::iter_bytes(input.as_bytes()).data)
            .finish();
        self.update(JimiEncoding::new(hajimi_tokens(), &freq))
            .expect("hajimi_tokens can't produce non-prefix-free encoding");
        self.encode_with_current_encoding(input)
    }

    fn update(&mut self, encoding: JimiEncoding<Bits8>) -> Result<(), LexemError> {
        self.encoding = encoding;
        self.encoder = self.encoding.encoder();
        self.decoder = self.encoding.decoder()?;
        Ok(())
    }

    fn decode(&self, input: &str) -> Result<String, JimiError> {
        self.decoder.decode_to_vec(input).map_or_else(
            |e| Err(e.unwrap_parent()),
            |x| Ok(String::from_utf8_lossy(&x[..]).to_string()),
        )
    }

    fn dump(&self) -> String {
        serde_json::to_string(&self.encoding).unwrap()
    }
}

#[component]
fn EncodingDisplay(enc: ReadSignal<JimiState>) -> impl IntoView {
    let enc_list = move || {
        BitsIter::<Bits8>::begin_zero()
            .map(move |b| (b, enc.with(|x| x.encoder.encode_bits(b).to_owned())))
    };

    view! {
        <table>
            <thead>
                <tr>
                    <th>字节</th>
                    <th>蜂蜜水</th>
                </tr>
            </thead>
            <tbody>
                <For
                    each=enc_list
                    key=|(_, s)| s.clone()
                    children=|(b, s)| {
                        view! {
                            <tr>
                                <td>{format!("{:X}", b.to_usize())}</td>
                                <td>{s}</td>
                            </tr>
                        }
                    }
                />
            </tbody>
        </table>
    }
}

#[component]
fn TextArea(
    rs: impl Fn() -> String + Send + Sync + Copy + 'static,
    set: impl Fn(String) + 'static,
) -> impl IntoView {
    view! {
        <textarea
            prop:value=move || rs()
            on:input:target=move |ev| set(ev.target().value())
        >
            {rs}
        </textarea>
    }
}

#[derive(Clone, Copy, Debug)]
enum LastModified {
    Plain,
    Encoded,
}

#[component]
fn App() -> impl IntoView {
    let (plain, set_plain) = signal("".to_string());
    let (encoded, set_encoded) = signal("".to_string());
    let (jimi, set_jimi) = signal(JimiState::default());
    let frequency_based = RwSignal::new(false);
    let (last_modified, set_last_modified) = signal(LastModified::Plain);
    let (decode_error, set_decode_error) = signal("".to_string());
    let (encoding_error, set_encoding_error) = signal("".to_string());

    let update_plain = move |p: String| {
        if frequency_based.get() {
            set_encoded(set_jimi.write().encode(&p));
        } else {
            set_encoded(jimi.read().encode_with_current_encoding(&p));
        }
        set_last_modified(LastModified::Plain);
        set_plain(p);
    };

    let decode = move |e: &str| -> String {
        match jimi.with(|x| x.decode(&e)) {
            Ok(r) => {
                set_decode_error("".to_string());
                r
            }
            Err(e) => {
                set_decode_error(format!("{:?}", e));
                "".to_string()
            }
        }
    };

    let update_encoded = move |e: String| {
        set_plain(decode(&e));
        set_encoded(e);
        set_last_modified(LastModified::Encoded);
    };

    let update_encoding_json = move |e: String| {
        if let Ok(enc) = serde_json::from_str(&e) {
            if let Err(err) = set_jimi.write().update(enc) {
                set_encoding_error(format!("{:?}", err));
                return;
            }
            set_encoding_error("".to_string());
            match last_modified.get() {
                LastModified::Plain => set_encoded(
                    jimi.read()
                        .encode_with_current_encoding(plain.read().as_str()),
                ),
                LastModified::Encoded => {
                    set_plain(decode(encoded.read().as_str()));
                }
            };
        } else {
            set_encoding_error("编码解析失败".to_string());
        }
    };

    let encoding_json = move || {
        let _ = plain.read();
        let _ = encoded.read();
        jimi.with(|x| x.dump())
    };

    view! {
        <div class="container">
            <header>
                <h1>哈基曼</h1>
            </header>

            <div class="main-content">
                <div class="text-areas">
                    <div class="radio-container">
                        <input type="checkbox"
                            bind:checked=frequency_based
                        />
                        <label for="frequency-based">Frequency Based</label>
                    </div>

                    <div class="input-group">
                        <div class="text-container plaintext">
                            <label for="plaintext">
                                {move || if matches!(last_modified(), LastModified::Plain) { "明文->蜜文" } else { "明文" }}
                            </label>
                            <TextArea rs=plain set=update_plain/>
                        </div>

                        <div class="text-container">
                            <label for="plaintext">
                                {move || if matches!(last_modified(), LastModified::Encoded) { "蜜文->明文" } else { "蜜文" }}
                            </label>
                            <TextArea rs=encoded set=update_encoded/>
                            <p>{decode_error}</p>
                        </div>

                        <div class="text-container">
                            <label for="encoded">编码</label>
                            <TextArea rs=encoding_json set=update_encoding_json/>
                            <p>{encoding_error}</p>
                        </div>
                    </div>

                </div>

                <div class="table-container">
                    <EncodingDisplay enc=jimi/>
                </div>
            </div>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App)
}
