import requests
import sys
import os
import yaml
import re
import toml
from bs4 import BeautifulSoup
from selenium import webdriver
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.chrome.options import Options
from webdriver_manager.chrome import ChromeDriverManager
from jinja2 import Environment, FileSystemLoader

OPENAPI_YAML_URL = 'https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml'
MODELS_DOCS_URL = 'https://platform.openai.com/docs/models'
PRICING_URL = 'https://openai.com/pricing'
ENUM_PATH = 'components.schemas.CreateChatCompletionRequest.properties.model.anyOf.1.enum'
DEFAULT_MODEL_PATH = 'components.schemas.CreateChatCompletionRequest.properties.model.example'
JINJA_TEMPLATE = 'code-gen/model.rs.j2'
OUTPUT_FILE = 'src/model.rs'
CARGO_TOML_FILE = 'Cargo.toml'

OLD_CODE = ''

def download_openapi_yaml(url):
    """Download the OpenAPI spec YAML file from a given URL"""
    response = requests.get(url)
    if response.status_code == 200:
        return response.text
    else:
        response.raise_for_status()

def extract_enum_info(yaml_content):
    """Extract enum information from the OpenAPI spec YAML content"""
    try:
        openapi_spec = yaml.safe_load(yaml_content)
        
        keys = ENUM_PATH.split('.') 
        current_data = openapi_spec
        for key in keys:
            if key in current_data:
                current_data = current_data[key]
            elif key.isdigit() and isinstance(current_data, list):
                current_data = current_data[int(key)]
            else:
                raise KeyError(f"Key path '{'.'.join(keys)}' not found")

        return current_data
    except yaml.YAMLError as e:
        print(f"🌋 Error parsing YAML: {e}")
        return None
    except Exception as e:
        print(f"🌋 Error extracting enum values: {e}")
        return None

def get_default_model_codename(yaml_content):
    try:
        openapi_spec = yaml.safe_load(yaml_content)
        keys = DEFAULT_MODEL_PATH.split('.') 
        default_model = openapi_spec
        for key in keys:
            default_model = default_model.get(key, None)
            if default_model is None:
                raise KeyError(f"Default model path '{'.'.join(keys)}' not found")
        return default_model
    except Exception as e:
        print(f"Error extracting default model: {e}")
        return None

def codename_to_enum(codename):
    """ Convert a codename string like 'gpt-3.5-turbo' to an enum name like 'Gpt35Turbo'. """
    codename = re.sub(r'[^a-zA-Z0-9-]', '', codename)
    parts = codename.split('-')
    enum_name = ''.join(part.capitalize() for part in parts)
    enum_name = re.sub(r'[^a-zA-Z0-9]', '', enum_name)
    return enum_name

def scrape_context_sizes(data_dict):
    """Scrape context sizes for the models listed in data_dict."""
    chrome_options = Options()
    chrome_options.add_argument("--headless")
    chrome_options.add_argument("user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
    service = Service(ChromeDriverManager().install())
    driver = webdriver.Chrome(service=service, options=chrome_options)

    driver.get(MODELS_DOCS_URL)
    WebDriverWait(driver, 10).until(EC.visibility_of_all_elements_located((By.TAG_NAME, "table")))

    soup = BeautifulSoup(driver.page_source, 'html.parser')
    driver.quit()

    for table in soup.find_all('table'):
        rows = table.find_all('tr')
        header = rows[0].find_all('th')
        if len(header) == 4:
            for row in rows[1:]:
                cols = row.find_all('td')
                codename = cols[0].get_text(strip=True)
                for model in data_dict:
                    if model['codename'] == codename:
                        tokens_text = cols[2].get_text(strip=True)
                        token_count = int(tokens_text.replace('tokens', '').replace(',', ''))
                        model['context_size'] = token_count
                        break

    results = [model for model in data_dict if 'context_size' in model]

    print(f"📝 Scraped context sizes for {len(results)} models")
    return results

def scrape_prices(data_dict):
    chrome_options = Options()
    chrome_options.add_argument("--headless")
    service = Service(ChromeDriverManager().install())
    driver = webdriver.Chrome(service=service, options=chrome_options)

    driver.get(PRICING_URL)

    soup = BeautifulSoup(driver.page_source, 'html.parser')
    driver.quit()

    pricing_data = {}
    tables = soup.find_all('table')
    for table in tables:
        rows = table.find_all('tr')
        for row in rows:
            cols = row.find_all('td')
            if len(cols) == 3 and 'gpt' in cols[0].get_text().strip().lower():
                model_codename = cols[0].get_text().strip().lower()
                input_price = float(re.search(r'\$(\d+\.\d+)', cols[1].get_text()).group(1))
                output_price = float(re.search(r'\$(\d+\.\d+)', cols[2].get_text()).group(1))
                pricing_data[model_codename] = {'input_price': input_price, 'output_price': output_price}

    for model in data_dict:
        codename_parts = model['codename'].split('-')
        for i in range(len(codename_parts), 0, -1):
            search_codename = '-'.join(codename_parts[:i])
            matches = [key for key in pricing_data if key.startswith(search_codename)]
            if matches:
                matches.sort(key=lambda x: (-len(x), x))
                closest_match = matches[0]
                break
        if not matches:
            closest_match = min(pricing_data.keys(), key=lambda x: len(x))

        model['prompt_cost'] = pricing_data[closest_match]['input_price']
        model['completion_cost'] = pricing_data[closest_match]['output_price']

    print(f"📝 Scraped prices for {len(data_dict)} models")
    return data_dict

def format_time(start, end):
    elapsed_time = end - start
    if elapsed_time < 1e-3:
       return f"{elapsed_time * 1e6:.2f}ns"
    elif elapsed_time < 1:
       return f"{elapsed_time * 1e3:.2f}ms"
    else:
       return f"{elapsed_time:.2f}s"

def render_rust_code(models, default_model_codename):
    """Render the Rust code from models using Jinja2"""
    env = Environment(
        loader=FileSystemLoader('.'),
        trim_blocks=True,
        lstrip_blocks=True
    )
    template = env.get_template(JINJA_TEMPLATE)

    rendered_code = template.render(models=models, default_model_codename=default_model_codename)
    
    return rendered_code

def increment_patch_version_if_model_changed(new_model_content):
    global OLD_CODE
    if OLD_CODE != new_model_content:
        with open(CARGO_TOML_FILE, 'r') as file:
            cargo_toml_data = toml.load(file)

        version = cargo_toml_data['package']['version']
        major, minor, patch = map(int, version.split('.'))
        patch += 1
        new_version = f"{major}.{minor}.{patch}"

        cargo_toml_data['package']['version'] = new_version

        with open(CARGO_TOML_FILE, 'w') as file:
            toml.dump(cargo_toml_data, file)
        print(f"📝 Incremented version from {version} to {new_version}")
    else:
        print("🤷 No changes to model code, not incrementing version")

def main():
    """Main function to run the download and extract process"""
    global OLD_CODE
    if os.path.exists(OUTPUT_FILE):
        with open(OUTPUT_FILE, 'r') as f:
            OLD_CODE = f.read()
        os.remove(OUTPUT_FILE)
        print(f"🗑️ Removed existing {OUTPUT_FILE}")
    try:
        yaml_content = download_openapi_yaml(OPENAPI_YAML_URL)
        valid_codenames = extract_enum_info(yaml_content)
        default_model_codename = get_default_model_codename(yaml_content)

        print(f"🔍 Found {len(valid_codenames)} valid codenames:")
        for codename in valid_codenames:
            print(f"     {codename}")
        
        data = [{ 'codename': codename, 'enumname': codename_to_enum(codename) } for codename in valid_codenames]

        data = scrape_context_sizes(data)
        data = scrape_prices(data)

        if data:
            rust_code = render_rust_code(data, default_model_codename)

            increment_patch_version_if_model_changed(rust_code)

            with open(OUTPUT_FILE, 'w') as f:
                f.write(rust_code)

            print(f"🦀 Saved Rust code to {OUTPUT_FILE}")
            sys.exit(0)
        else:
            print("No data captured, no Rust code generated.🚫🦀")
            sys.exit(1)
    except requests.RequestException as e:
        print(f"Request failed: {e}")

if __name__ == "__main__":
    main()
