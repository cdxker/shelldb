## Shell DB

A database that runs shell commands across files

Currently supported features:
- `grep`
- `ls` Coming soon
- `awk` Coming soon

ShellDB additionally supports file `tagging`.

Tagging a file allows for building complex `filter`s that join multiple tags together.

## Running ShellDB

```
docker run cdxker/shelldb -p 8080:8080
```

## Architecture

- Files are stored within `${dataDir}/dataset_id/path/to/file`
- Tagged files are files are stored within `${dataDir}/dataset_id/___tag/path/to/file`

## Indexing files

To index a file with ShellDB use the [`index`](/api-reference/index/post-apiindex) route, sending the full file as a payload.

The `dataseet` field is the `index` you will be adding it into, if the `index` doesn't exist, one will be auto created for you.

```sh
 curl -X POST http://127.0.0.1:8080/api/index \
  -H "Content-Type: application/json" \
  -d '{
    "dataset": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "test.txt",
    "file_payload": "override",
    "tags": []
  }'
```

### Adding Tags

```sh
 curl -X POST http://127.0.0.1:8080/api/index \
  -H "Content-Type: application/json" \
  -d '{
    "dataset": "550e8400-e29b-41d4-a716-446655440000",
    "filename": "test.txt",
    "file_payload": "override",
    "tags": []
  }'
```

### Indexing files from Github

Coming Soon

## Searching

```sh
curl -X POST http://127.0.0.1:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "dataset": "550e8400-e29b-41d4-a716-446655440000",
    "flags": "-r hello",
    "tags": []
  }'
```

### Search filters

Filtering is currently only supported by tags. 

To filter with a tag send a list of tags like this

```sh
curl -X POST http://127.0.0.1:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "dataset": "550e8400-e29b-41d4-a716-446655440000",
    "flags": "-r private -U",
    "tags": ["private"]
  }'
```
