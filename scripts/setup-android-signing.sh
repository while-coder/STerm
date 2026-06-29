#!/usr/bin/env bash
# 在 `tauri android init` 之后、`tauri android build` 之前运行。
# 解码 keystore、写入 keystore.properties，并向生成的 build.gradle.kts 注入 release 签名配置。
# 幂等：重复运行不会损坏文件。
#
# 依赖的环境变量（来自 GitHub Actions secrets）：
#   ANDROID_KEY_ALIAS    签名别名（如 upload）
#   ANDROID_KEY_PASSWORD keystore/key 密码
#   ANDROID_KEY_BASE64   keystore（.jks）的 base64 编码
#   RUNNER_TEMP          运行器临时目录（GitHub Actions 自动提供）
set -euo pipefail

ANDROID_DIR="src-tauri/gen/android"
GRADLE_FILE="${ANDROID_DIR}/app/build.gradle.kts"
KEYSTORE_PATH="${RUNNER_TEMP:-/tmp}/keystore.jks"

if [[ ! -f "${GRADLE_FILE}" ]]; then
  echo "::error::${GRADLE_FILE} 不存在，请先执行 'tauri android init'"
  exit 1
fi

: "${ANDROID_KEY_ALIAS:?需要设置 ANDROID_KEY_ALIAS}"
: "${ANDROID_KEY_PASSWORD:?需要设置 ANDROID_KEY_PASSWORD}"
: "${ANDROID_KEY_BASE64:?需要设置 ANDROID_KEY_BASE64}"

# 1) 解码 keystore 到临时目录
base64 -d <<< "${ANDROID_KEY_BASE64}" > "${KEYSTORE_PATH}"

# 2) 写 keystore.properties
cat > "${ANDROID_DIR}/keystore.properties" <<EOF
keyAlias=${ANDROID_KEY_ALIAS}
password=${ANDROID_KEY_PASSWORD}
storeFile=${KEYSTORE_PATH}
EOF

# 3) 向 build.gradle.kts 注入签名配置（幂等）
if grep -q 'signingConfigs.getByName("release")' "${GRADLE_FILE}"; then
  echo "签名配置已存在，跳过注入。"
  exit 0
fi

PYTHON="$(command -v python3 || command -v python)"
if [[ -z "${PYTHON}" ]]; then
  echo "::error::未找到 python3/python，无法注入签名配置"
  exit 1
fi

"${PYTHON}" - "${GRADLE_FILE}" <<'PY'
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as f:
    src = f.read()

# a) 顶部插入 import（若缺失）
if "import java.io.FileInputStream" not in src:
    src = "import java.io.FileInputStream\n" + src

signing_block = '''    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            }
            keyAlias = keystoreProperties["keyAlias"] as String
            keyPassword = keystoreProperties["password"] as String
            storeFile = file(keystoreProperties["storeFile"] as String)
            storePassword = keystoreProperties["password"] as String
        }
    }

'''

# b) 在 `buildTypes {` 之前插入 signingConfigs 块
marker = re.search(r'^([ \t]*)buildTypes\s*\{', src, re.MULTILINE)
if not marker:
    sys.exit("未在 build.gradle.kts 中找到 buildTypes 块")
indent = marker.group(1)
block = "\n".join(indent + line if line else line for line in signing_block.splitlines()) + "\n"
src = src[:marker.start()] + block + src[marker.start():]

# c) 在 release 构建类型中引用签名配置
def add_signing(match):
    body = match.group(0)
    if "signingConfig" in body:
        return body
    return body.replace("{", '{\n            signingConfig = signingConfigs.getByName("release")', 1)

src = re.sub(r'getByName\("release"\)\s*\{', add_signing, src, count=1)

with open(path, "w", encoding="utf-8") as f:
    f.write(src)

print("已注入 Android release 签名配置。")
PY
