[![CI](https://github.com/lennartkloock/simple-password-vault/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/lennartkloock/simple-password-vault/actions/workflows/ci.yml)

# 🔐 Simple Password Vault

## 🔌 Setup

### 📂 Download

First, please create a new folder. The folder name doesn't matter, using `spv` here.

```shell
mkdir spv && cd spv
```

Then, download the latest tar archive from [releases](https://github.com/lennartkloock/simple-password-vault/releases)
and unpack it.

```shell
TODO
```

Now, the new folder should contain the following items:

- the server binary `simple-password-vault`
- the configuration file `Rocket.toml`
- a folder called `public` (This folder contains all HTML, CSS and images)

### 🔧 Configuration

To make sure the password vault can encrypt all stored passwords securely, please generate a new RSA keypair first. The
keypair must be encoded in PEM PKCS#1.

```shell
openssl genrsa -out keys/rsakey.pem 2048
openssl rsa -in keys/rsakey.pem -outform PEM -pubout -out keys/rsapubkey.pem
```

To configure the password vault, please edit the `Rocket.toml` file.
Since the password vault is built on top of the [Rocket](https://rocket.rs) framework, the configuration format and all
of [rocket's configuration parameters](https://rocket.rs/v0.5-rc/guide/configuration/#overview) can be used to further
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

**⚠️ Attention**: Be aware that every file placed in the folder specified in `static_dir` or any sub folder will be
publicly reachable through the webserver!

### Example configuration
```toml
[release]
address = "0.0.0.0"
name = "Password Vault"
db_url = "mariadb://root:password@localhost:3306/vault_db"
template_dir = "public/templates"
```

## 📷 Screenshots

## 📜 License

This software is licensed under the terms of the [MIT license](https://github.com/lennartkloock/simple-password-vault/blob/master/LICENSE).

<hr>

&copy; 2021 Lennart Kloock
<br>
[Free icons by Streamline](https://streamlinehq.com)
