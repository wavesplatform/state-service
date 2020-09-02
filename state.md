# [todo name] v1 protocol

What [todo name] v1 is:

- Protocol for storing data in the Waves key-value state
- Search DSL for writing queries
- Service that indexes Waves state and provides search API using the Search DSL

## Protocol

### Key format:

`#{int_value}${string_value}...`

`#` and `$` act as typed separators.

Keys are split by the separators `$` or `#`, then stored as as separate _fragments_. Each fragment is independently indexed. Fragments are identified by their position, starting with 0.

Fragments are typed. Type is decided by the used separator:

- `$` for strings
- `#` for signed 64-bit integers

### Example:

Key: `$order$height#1000`

For such key a fragment 0 would be a string `order`, fragment 1 is a string `height` and fragment 2 is an integer `1000`.

- Empty strings are allowed
- Negative integers are allowed
- Up to N [todo decide] fragments in each key are allowed

## Search DSL

### Specification

#### Query object:

```json
{
  "filter": {}, // filters object
  "sort": {}, // sort object
  "limit": 1000,
  "offset": 0
}
```

#### Filter object:

##### Single-term filter:

Entries can be filtered by either a `fragment` or an `address`.

###### Fragment filter

```json
{
  "fragment": {
    "position": 0,
    "type": "string",
    "operation": "eq",
    "value": "order"
  }
}
```

Supported operations: `eq`, `ne`, `lt`, `gt`, `lte`, `gte`.

###### Address filter

```json
{
  "address": {
    "operation": "eq",
    "value": "9u0as9dufopmwidsa09pfuamsldf9jas"
  }
}
```

Eq operation only.

###### Key filter

Filters by full key

```json
{
  "key": {
    "operation": "eq",
    "value": "$order$height#1000"
  }
}
```

Eq operation only.

###### Value filter

```json
{
  "value": {
    "type": "string",
    "operation": "eq",
    "value": "9u0as9dufopmwidsa09pfuamsldf9jas"
  }
}
```

Eq operation only.

##### IN filter:

Maps to SQL IN

```json
{
  "in": {
    "properties": [
      { "address": {} },
      { "key": {} },
      {
        "fragment": {
          "position": 2,
          "type": "integer"
        }
      }
    ],
    "values": [
      ["addr1", "key1", 10000],
      ["addr1", "key2", 10001],
      ["addr2", "key1", 10002]
    ]
  }
}
```

##### Composite filters:

Composite filters are achieved using `or`, `and` operators.

```json
{
  "and": [
    {
      // nested filter object
    },
    {
      // nested filter object
    }
  ]
}
```

```json
{
  "or": [
    {
      // nested filter object
    },
    {
      // nested filter object
    }
  ]
}
```

### Example

`POST /search`

```json
{
  "filter": {
    "and": [
      {
        "and": [
          {
            "fragment": {
              "position": 0,
              "type": "string",
              "operation": "eq",
              "value": "order"
            }
          },
          {
            "fragment": {
              "position": 1,
              "type": "string",
              "operation": "eq",
              "value": "height"
            }
          },
          {
            "fragment": {
              "position": 2,
              "type": "integer",
              "operation": "gte",
              "value": 1900000
            }
          }
        ]
      },
      {
        "or": [
          {
            "address": {
              "value": "contract_address_1"
            }
          },
          {
            "address": {
              "value": "contract_address_2"
            }
          },
          {
            "address": {
              "value": "contract_address_2"
            }
          }
        ]
      }
    ]
  },
  "sort": [
    {
      "fragment": {
        "position": 2,
        "direction": "desc"
      }
    }
  ],
  "limit": 1000,
  "offset": 0
}
```

Response:

```json
{
  "entries": [
    {
      "address": "asadfasdfasdfewqer23qe12qwadf",
      "key": "$order$height#1029",
      "fragments": [
        {
          "type": "string",
          "value": "order"
        },
        {
          "type": "string",
          "value": "height"
        },
        {
          "type": "integer",
          "value": "1029"
        }
      ],
      "value": {
        "type": "string",
        "value": "SOME_STRING_VALUE"
      }
    }
    // other entries
  ],
  "has_next_page": false
}
```
