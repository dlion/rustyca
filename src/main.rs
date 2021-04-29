use scraper::{Html, Selector};
use yaml_rust::YamlLoader;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::error::Error;

fn main() {
    let name = r#"
__________                __
\______   \__ __  _______/  |_ ___.__. ____ _____
 |       _/  |  \/  ___/\   __<   |  |/ ___\\__  \
 |    |   \  |  /\___ \  |  |  \___  \  \___ / __ \_
 |____|_  /____//____  > |__|  / ____|\___  >____  /
        \/           \/        \/         \/     \/
"#;
    println!("{}", name);

    let content = match get_yaml_content() {
        Ok(content) => content,
        Err(_) =>{
                println!("Can't open the yaml configuration file");
                exit(1)
            }
    };

    let (mobile_number, password) = match get_credentials_from_the_yaml(&content) {
        Ok((mobile_phone, password)) => (mobile_phone, password),
        Err(_) => {
            println!("Can't get the credentials from the yaml configuration file");
            exit(1)
        }
    };

    let parsed_html = match get_body(mobile_number, password) {
        Ok(response) => Html::parse_document(&response),
        Err(_) => {
            println!("Can't get the body response from the lycamobile.es website");
            exit(1)
        }
    };

    let money_balance = get_money_balance(&parsed_html);
    let internet_balance = get_internet_balance(&parsed_html);
    let expiration_date = get_expiration_date(&parsed_html);

    println!("Money Balance: {}\nInternet Balance: {}\nExpiration Date: {}", money_balance, internet_balance, expiration_date);
}

fn get_credentials_from_the_yaml(content: &str) -> Result<(String, String), Box<dyn Error>> {
    let configs = YamlLoader::load_from_str(content)?;
    let config = &configs[0];
    let mobile_number = config["mobile-phone-number"].as_i64().expect("Can't get the mobile phone number").to_string();
    let password = config["password"].as_str().expect("Can't get the password");
    Ok((mobile_number, String::from(password)))
}

fn get_yaml_content() -> Result<String, Box<dyn Error>> {
    let home = std::env::var("HOME")?;
    let mut config_file = File::open(format!("{}/.config/.rustyca_config.yml", home))?;
    let mut content = String::new();
    config_file.read_to_string(&mut content)?;
    Ok(content)
}

fn get_body(mobile_number: String, password: String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder().cookie_store(true).build()?;
    client.post("https://www.lycamobile.es/wp-admin/admin-ajax.php")
        .form(&[("action", "lyca_login_ajax"), ("method", "login"), ("mobile_no", &mobile_number), ("pass", &password)])
        .send()?;

    let response = client.get("https://www.lycamobile.es/es/my-account/").send()?;
    let text_response = response.text()?;
    Ok(text_response)
}

fn get_expiration_date(parsed_html: &Html) -> String {
    //E.g. <p class=\"bdl-balance\"><span>InternacionalSaldo al 28-04-2021</span> <span>| InternacionalVálido hasta 07-05-2021</span>
    let p_element = get_element_from(parsed_html, "p.bdl-balance > span");
    let expiration_date = p_element.split("hasta").nth(1).expect("Can't get the expiration date correctly");
    expiration_date.trim_start().to_string()
}

fn get_money_balance(parsed_html: &Html) -> String {
    //E.g. <span class=\'myaccount-lowbalance\'>€0.03\n
    let div_element = get_element_from(parsed_html, "span.myaccount-lowbalance");
    let mut split = div_element.lines();
    let money_balance = split.next().expect("Can't get the money balance correctly");
    money_balance.to_string()
}

fn get_internet_balance(parsed_html: &Html) -> String {
    //E.g. <div class=\"bdl-mins\">\n\n5.66GB</div>
    let span_element = get_element_from(parsed_html, "div.bdl-mins");
    let internet_balance = span_element.get(2..).expect("Can't get the internet balance correctly");
    internet_balance.to_string()
}

fn get_element_from(parsed_html: &Html, selector: &str) -> String {
    let selector = &Selector::parse(selector).expect("Error during the parsing using the given selector");
    parsed_html
        .select(selector)
        .flat_map(|el| el.text())
        .collect()
}
