# Алгоритм публикации образа Bus9 в Docker Hub

## Предварительные требования

1. **Аккаунт на Docker Hub**: https://hub.docker.com
2. **Docker установлен** на вашей машине
3. **Авторизация в Docker Hub** через командную строку

## Пошаговый алгоритм

### Шаг 1: Авторизация в Docker Hub

```bash
docker login
```

Введите ваш Docker Hub username и password.

### Шаг 2: Определить имя образа

Формат: `{dockerhub-username}/{image-name}`

Примеры:
- `denyzhirkov/bus9` (если ваш username - denyzhirkov)
- `yourusername/bus9`

### Шаг 3: Сборка образа

Соберите образ с тегом версии и latest:

```bash
# Читаем версию из version.json
VERSION=$(grep -o '"version": "[^"]*"' version.json | cut -d'"' -f4)
DOCKER_USERNAME="denyzhirkov"  # Замените на ваш username

# Собираем образ с версией
docker build -t ${DOCKER_USERNAME}/bus9:${VERSION} .

# Собираем образ с тегом latest
docker build -t ${DOCKER_USERNAME}/bus9:latest .
```

Или одной командой:

```bash
VERSION=$(grep -o '"version": "[^"]*"' version.json | cut -d'"' -f4)
DOCKER_USERNAME="denyzhirkov"

docker build -t ${DOCKER_USERNAME}/bus9:${VERSION} -t ${DOCKER_USERNAME}/bus9:latest .
```

### Шаг 4: Проверка образа

Убедитесь, что образ создан:

```bash
docker images | grep bus9
```

### Шаг 5: Публикация образа

Отправьте образ в Docker Hub:

```bash
# Публикуем версию
docker push ${DOCKER_USERNAME}/bus9:${VERSION}

# Публикуем latest
docker push ${DOCKER_USERNAME}/bus9:latest
```

Или обе версии сразу:

```bash
docker push ${DOCKER_USERNAME}/bus9:${VERSION}
docker push ${DOCKER_USERNAME}/bus9:latest
```

## Автоматизированный скрипт

Создайте файл `publish_docker.sh`:

```bash
#!/bin/bash

set -e

# Настройки
DOCKER_USERNAME="denyzhirkov"  # Замените на ваш Docker Hub username

# Читаем версию
if [ ! -f "version.json" ]; then
    echo "Error: version.json not found"
    exit 1
fi

VERSION=$(grep -o '"version": "[^"]*"' version.json | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
    echo "Error: Could not read version from version.json"
    exit 1
fi

echo "Building Docker image for version: $VERSION"

# Сборка образа
docker build -t ${DOCKER_USERNAME}/bus9:${VERSION} -t ${DOCKER_USERNAME}/bus9:latest .

echo "Pushing images to Docker Hub..."

# Публикация
docker push ${DOCKER_USERNAME}/bus9:${VERSION}
docker push ${DOCKER_USERNAME}/bus9:latest

echo "Successfully published ${DOCKER_USERNAME}/bus9:${VERSION} and ${DOCKER_USERNAME}/bus9:latest"
```

Использование:

```bash
chmod +x publish_docker.sh
./publish_docker.sh
```

## Интеграция с bump_version.sh

Можно автоматизировать публикацию после обновления версии. Добавьте в конец `bump_version.sh`:

```bash
# Опционально: автоматическая публикация в Docker Hub
read -p "Publish to Docker Hub? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    ./publish_docker.sh
fi
```

## Проверка публикации

После публикации проверьте на Docker Hub:

1. Откройте https://hub.docker.com/r/{your-username}/bus9
2. Убедитесь, что образы появились
3. Проверьте теги (версия и latest)

## Тестирование опубликованного образа

```bash
# Запуск образа с версией
docker run -p 8080:8080 ${DOCKER_USERNAME}/bus9:${VERSION}

# Запуск образа latest
docker run -p 8080:8080 ${DOCKER_USERNAME}/bus9:latest
```

## Рекомендации

1. **Всегда публикуйте с версией** - это позволяет откатываться к предыдущим версиям
2. **Обновляйте latest** - для удобства пользователей
3. **Используйте семантическое версионирование** - major.minor.patch
4. **Проверяйте образ перед публикацией** - запустите локально
5. **Документируйте изменения** - обновляйте README при значительных изменениях

## Troubleshooting

### Ошибка: "denied: requested access to the resource is denied"

**Решение**: Убедитесь, что вы авторизованы (`docker login`) и используете правильный username.

### Ошибка: "unauthorized: authentication required"

**Решение**: Выполните `docker login` заново.

### Ошибка: "tag does not exist"

**Решение**: Убедитесь, что образ собран с правильным тегом перед push.
