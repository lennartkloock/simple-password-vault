[![CI](https://github.com/lennartkloock/simple-password-vault/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/lennartkloock/simple-password-vault/actions/workflows/ci.yml)

# 🔐 Simple Password Vault

## 🔌 Setup

### 📂 Download

First, download the latest tar archive from [releases](https://github.com/lennartkloock/simple-password-vault/releases)
and unpack it.

```shell
curl -sL --url 'https://github.com/lennartkloock/simple-password-vault/releases/download/v0.2.0/simple_password_vault.tar.gz' --output 'simple_password_vault.tar.gz'
tar -xzf simple_password_vault.tar.gz
cd spv
```

Now, the new folder should contain the following items:

- the server binary `simple-password-vault`
- the default configuration file `Rocket.toml`
- a folder called `public` (This folder contains all HTML, CSS and images)

### 🔧 Configuration

To make sure the password vault can encrypt all stored passwords securely, please generate a new RSA keypair.
The keypair must be encoded in PEM PKCS#1.

```shell
openssl genrsa -out keys/rsakey.pem 2048
openssl rsa -in keys/rsakey.pem -outform PEM -pubout -out keys/rsapubkey.pem
```

To configure the password vault, please edit the `Rocket.toml` file. Since the password vault is built on top of
the [Rocket](https://rocket.rs) framework, the configuration format and all
of [rocket's configuration parameters](https://rocket.rs/v0.5-rc/guide/configuration) can be used to further
configure the password vault.

Additionally, the following keys can be used:

| Key                            | Description                                                                          | Default value          | Example value                                       |
|--------------------------------|--------------------------------------------------------------------------------------|------------------------|-----------------------------------------------------|
| `name`                         | The application's name which will be displayed in the top left corner                | `"Password Vault"`     | `"My cool password safe"`                           |
| `db_url`                       | The MySQL/MariaDB database url                                                       |                        | `"mariadb://root:password@localhost:3306/vault_db"` |
| `static_dir`                   | The directory of all static files (css, fonts and images)                            | `"public/static"`      | `"public/static"`                                   |
| `token_length`                 | The length that will be used when generating new authorization tokens                | `32`                   | `64`                                                |
| `token_validity_duration_secs` | The duration in seconds that one authorization token (login session) needs to expire | `86400` (1 day)        | `604800` (7 days)                                   |
| `public_key_path`              | The path to the public encryption key (relative to the binary)                       | `"keys/rsapubkey.pem"` | `"keys/key_pub.pem"`                                |
| `private_key_path`             | The path to the private encryption key (relative to the binary)                      | `"keys/rsakey.pem"`    | `"keys/key.pem"`                                    |

**⚠️ Attention**: Be aware that every file placed in the folder specified in `static_dir` or any sub folders will be
publicly reachable through the webserver!

### Example configuration

```toml
[default]
address = "0.0.0.0"
port = 80
name = "Password Vault"
db_url = "mariadb://root:password@localhost:3306/vault_db"
template_dir = "public/templates"
```

### 🚀 Run it

**Note**: You will need to create the specified database, otherwise the next step will fail. (For example with: `CREATE DATABASE vault_db;`)

After editing the `Rocket.toml` file according to your wishes, you can run the binary:
```shell
./simple-password-vault
```

When everything worked you should be able to navigate to the specified port in your web browser.

In the following you have to set a new password for the admin account.
This only happens at the first launch of the application or when all admin accounts were deleted.
After logging in with your newly created admin account, the password vault is ready to be used.

## 📷 Screenshots

![no-table](https://user-images.githubusercontent.com/39778085/146641984-09915746-42c1-4b6e-9609-a2324e1cdae4.png)
![important-passwords](https://user-images.githubusercontent.com/39778085/146641990-dc83ac57-82c8-4668-9f21-0bf15a1dc9e9.png)
![customer-passwords](https://user-images.githubusercontent.com/39778085/146641991-381d4635-ef3b-482f-8a1d-aabc31e3094d.png)

## 📜 License

This software is licensed under the terms of
the [MIT license](https://github.com/lennartkloock/simple-password-vault/blob/master/LICENSE).

<hr>

&copy; 2021 Lennart Kloock
<br>
[Free icons by Streamline](https://streamlinehq.com)
