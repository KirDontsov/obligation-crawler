#!/bin/bash
#$ cat start.sh

echo "├➔ Проверяем от какого пользователя запускаем скрипт   | $(whoami)"
echo "├➔ Проверяем директорию в которой находимся            | $(pwd)"
echo "├➔ Проверяем обновления                                | $(git pull)"
echo "├➔ Компилируем crawler                                 |"
$(cargo build -r)

# запускаем драйвер браузера

chromedriver_drv=$(pgrep chromedriver)
if [ -z $chromedriver_drv ]
then
echo "├➔ Запускаем драйвер для работы браузера               |"
./chromedriver --port=9515 --whitelisted-ips="" --allowed-origins=* --disable-gpu --dns-prefetch-disable --disable-extensions --no-sandbox enable-automation --host=0.0.0.0 &
sleep 5
echo "├➔ Драйвер запущен                                     | $(pgrep chromedriver)"
else
echo "├➔ Драйвер для работы браузера уже запущен             | $(pgrep chromedriver)"
fi
echo "└➔ Запускаем СКАНЕР! (НЕ стартует? Повторите попытку!) | crawler"
target/release/obligation-crawler
