# Gaboja: CLI helper for solving BOJ problems

## Gaboja를 사용하기 전에

다음의 프로그램들이 설치되어 있어야 합니다.

* Firefox
    * 윈도우나 맥 사용자의 경우, [공식 홈페이지](https://www.mozilla.org/en-US/firefox/new/)에서 다운받아 설치할 수 있습니다.
    * 데비안이나 우분투 사용자의 경우, [mozillateam PPA](https://launchpad.net/~mozillateam/+archive/ubuntu/ppa)를 통하여 설치하는 것을 추천합니다.
* geckodriver
    * 러스트 툴체인이 설치되어 있다면 `cargo install geckodriver`로 설치할 수 있습니다.
    * 아닌 경우, [github releases](https://github.com/mozilla/geckodriver/releases)에서 다운받은 실행 파일을 `PATH`에 포함된 폴더에 넣거나 (`/usr/local/bin` 등), 실행 파일을 저장한 폴더를 `PATH`에 추가해 주세요.

## 설치 방법

* 러스트 툴체인이 설치되어 있다면 `cargo install gaboja`를 실행하여 설치할 수 있습니다.
* 아닌 경우, 이 repo의 releases 페이지에서 가장 나중에 올라온 바이너리를 받아 geckodriver와 같은 방법으로 설치하시면 됩니다.
    * 윈도우 빌드의 경우 바이러스로 인식이 된다거나 잘 동작하지 않거나 할 수 있습니다. 그런 경우에는 issue를 열어 주세요.
* 설치하고 실행하기 전에 터미널을 재시작해야 할 수 있습니다.

## 사용 방법

터미널을 켜서, 문제를 푸는 코드를 작성할 폴더 위치에서 gaboja를 실행합니다. 조금 기다리면 `BOJ >`라는 프롬프트가 나타납니다. 여기서 `help`를 입력하여 사용 가능한 커맨드를 확인할 수 있습니다.

`<VAR>`는 단순 매개변수 (예시: `1234`), `<c=VAR>`는 키워드 매개변수 (예시: `c=1234`)입니다. 키워드 매개변수끼리는 순서를 바꿔도 동작합니다.

매개변수에 공백을 포함한 문자열을 넣으려면 `'abc def'` 또는 `c='abc def'`처럼 입력할 수 있고, `'` 대신 `"`도 동작합니다. 매개변수 자리에 `$VAR`를 넣으면 환경 변수 `VAR`가 그 자리에 들어갑니다.

```
# 지원하는 커맨드의 목록을 간략하게 알려 줍니다.
help

# 로그인 쿠키를 입력합니다. 이것을 먼저 실행하지 않으면 거의 모든 커맨드가 동작하지 않습니다.
# 내부적으로 브라우저를 띄우는 등의 동작이 포함되어 있어 수 초에서 수십 초 가량 걸릴 수 있습니다.
set credentials <BOJAUTOLOGIN> <ONLINEJUDGE>

# 제출 언어, 제출 파일명, 빌드 커맨드, 실행 커맨드, 커스텀 입력 파일명을 설정합니다.
# 제출 언어는 BOJ 제출 언어 이름과 정확히 일치하지 않으면 동작하지 않을 수 있습니다.
# 파일명과 커맨드는 `{}` 또는 `{c}`를 포함할 수 있습니다. (`c`는 임의의 1글자)
# 문제마다 다른 파일에 풀이를 작성하는 경우, `{}` 부분에 문제 번호를 삽입해 줍니다.
# 대회 문제의 경우 `{}`는 `(대회 번호)_(문제 번호)`, `{c}`는 `(대회 번호)c(문제 번호)`로 치환됩니다.
set lang <LANG>
set file <FILE>
set init <INITCMD>
set build <BUILD>
set cmd <CMD>
set input <INPUT>

# 문제를 로드하고 기본 정보를 출력합니다. 대회 문제는 (대회 번호)/(문제 번호)로 입력합니다.
# init 커맨드가 설정되어 있으면 init을 실행합니다. 문제 번호별 폴더나 소스 파일을 생성하는 데 사용할 수 있습니다.
prob <PROB>

# 여러 개의 변수를 boj.toml에 정의된 preset으로 교체합니다.
# credential이 변경되면 새로 로그인을 진행하고, 문제가 로드된 상태에서 init이 변경되면 새로운 init을 즉시 실행합니다.
# 문제에 따라 언어를 바꿔가면서 풀 때 유용합니다.
preset <NAME>

# 아래의 커맨드에서 매개변수를 생략하면 위의 set 커맨드로 설정된 값을 사용합니다.
# 자주 쓰는 값들을 모두 set 해놓고 build, run, test, submit 등으로 간단하게 사용할 수 있습니다.

# 주어진 커맨드를 사용하여 소스를 빌드합니다. (C++, Rust 등의 경우 사용)
# 예시: build 'cargo build --release'
build [BUILD]

# 주어진 커맨드를 사용하여 소스를 실행하고, 주어진 입력 파일을 넣어 결과를 확인합니다.
# 문제 유형에 따라 동작이 달라지거나 동작하지 않을 수 있습니다.
# 예를 들어, 인터랙티브 문제는 유저가 입력을 키보드로 넣는 방식으로 실행되고, 함수 구현 문제는 동작하지 않습니다.
# 예시: run i='input.txt' c='./target/release/main'
run [i=INPUT] [c=CMD]

# 문제의 예제 입력을 넣어 예제 출력과 일치하는지 확인합니다.
# 문제 유형에 따라 동작이 달라지거나 동작하지 않을 수 있습니다.
# 예를 들어, 인터랙티브 문제는 동작하지 않고, 스페셜 저지는 결과만 보여줍니다.
test [c=CMD]

# 소스를 문제에 제출하고 결과를 확인합니다.
submit [l=LANG] [f=FILE]

# firefox와 geckodriver를 끄고 gaboja를 종료합니다.
exit
```

## `boj.toml`

gaboja를 실행하는 폴더에 `boj.toml`이라는 파일을 두어 시작 시 자동 실행할 커맨드와 preset을 설정할 수 있습니다.

```toml
# start: multiline string
# 시작 시에 줄 단위로 자동으로 실행됩니다.
start = '''
set credentials $BUB_BOJAUTOLOGIN $BUB_ONLINEJUDGE
preset rust
'''

# preset: array of objects
# name은 필수 항목이며, 나머지는 모두 생략할 수 있습니다.
[[preset]]
name = 'rust'
init = ''
build = 'cargo oj && cargo build --release --bin main'
cmd = './target/release/main'
input = 'input.txt'
lang = 'Rust 2021'
file = 'src/bin/main.rs'

[[preset]]
name = 'py'
init = ''
build = ''
cmd = 'python src.py'
input = 'input.txt'
lang = 'Python 3'
file = 'src.py'
```
