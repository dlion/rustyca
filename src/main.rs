use scraper::{Html, Selector};
use yaml_rust::YamlLoader;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::error::Error;
use reqwest::Url;

fn main() {
    print_title();

    let content = match get_yaml_content() {
        Ok(content) => content,
        Err(_) => {
                println!("Can't open the yaml configuration file");
                exit(1)
            }
    };

    let (mobile_number, password) = match get_credentials_from_the_yaml(&content) {
        Ok((mobile_phone, password)) => (mobile_phone, password),
        Err(e) => {
            println!("Can't get the credentials from the yaml configuration file: {}", e);
            exit(1)
        }
    };

    let parsed_html = match get_body(mobile_number, password) {
        Ok(response) => Html::parse_document(&response),
        Err(e) => {
            println!("Can't get the body response from the lycamobile.es website: {}", e);
            exit(1)
        }
    };

    let money_balance = get_money_balance(&parsed_html);
    let internet_balance = get_internet_balance(&parsed_html);
    let expiration_date = get_expiration_date(&parsed_html);

    println!("Money Balance: {}\nInternet Balance: {}\nExpiration Date: {}", money_balance, internet_balance, expiration_date);
}

fn print_title() {
    let name = r#"
__________                __
\______   \__ __  _______/  |_ ___.__. ____ _____
 |       _/  |  \/  ___/\   __<   |  |/ ___\\__  \
 |    |   \  |  /\___ \  |  |  \___  \  \___ / __ \_
 |____|_  /____//____  > |__|  / ____|\___  >____  /
        \/           \/        \/         \/     \/
"#;
    println!("{}", name);
}

fn get_credentials_from_the_yaml(content: &str) -> Result<(String, String), Box<dyn Error>> {
    let configs = YamlLoader::load_from_str(content)?;
    let config = &configs[0];
    let mobile_number = config["mobile-phone-number"].as_i64()
        .ok_or("Can't parse the mobile phone number")
        .map_err(|err| err.to_string())?;
    let password = config["password"].as_str()
        .ok_or("Can't parse the password")
        .map_err(|err| err.to_string())?;
    Ok((mobile_number.to_string(), String::from(password)))
}

fn get_yaml_content() -> Result<String, Box<dyn Error>> {
    let home = std::env::var("HOME")?;
    let mut config_file = File::open(format!("{}/.config/.rustyca_config.yml", home))?;
    let mut content = String::new();
    config_file.read_to_string(&mut content)?;
    Ok(content)
}

fn get_body(mobile_number: String, password: String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .build()?;
    client.post("https://www.lycamobile.es/wp-admin/admin-ajax.php")
        .form(&[
            ("action", "lyca_login_ajax"),
            ("method", "login"),
            ("mobile_no", &mobile_number),
            ("pass", &password)
        ])
        .send()?;

    let account_url = "https://www.lycamobile.es/es/my-account/";
    let response = client.get(account_url).send()?;
    return if response.url().eq(&Url::parse(account_url)?) {
        Ok(response.text()?)
    } else {
        Err("Can't log in".into())
    }
}

fn get_expiration_date(parsed_html: &Html) -> String {
    //E.g. <p class=\"bdl-balance\"><span>InternacionalSaldo al 28-04-2021</span> <span>| InternacionalVálido hasta 07-05-2021</span>
    get_element_from(parsed_html, "p.bdl-balance > span")
        .split("hasta")
        .nth(1)
        .unwrap_or_else(||"Can't get the expiration date correctly")
        .trim_start()
        .to_string()
}

fn get_money_balance(parsed_html: &Html) -> String {
    //E.g. <span class=\'myaccount-lowbalance\'>€0.03\n
    get_element_from(parsed_html, "span.myaccount-lowbalance")
        .lines()
        .next()
        .unwrap_or_else(||"Can't get the money balance correctly")
        .to_string()
}

fn get_internet_balance(parsed_html: &Html) -> String {
    //E.g. <div class=\"bdl-mins\">\n\n5.66GB</div>
    get_element_from(parsed_html, "div.bdl-mins")
        .get(2..)
        .unwrap_or_else(||"Can't get the internet balance correctly")
        .to_string()
}

fn get_element_from(parsed_html: &Html, selector: &str) -> String {
    let selector = &Selector::parse(selector)
        .expect("Error during the parsing using the given selector");
    parsed_html
        .select(selector)
        .flat_map(|el| el.text())
        .collect()
}
