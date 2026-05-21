(function() {
  let api
  htmx.defineExtension('json-enc', {
    init: function(apiRef) {
      api = apiRef
    },

    onEvent: function(name, evt) {
      if (name === 'htmx:configRequest') {
        evt.detail.headers['Content-Type'] = 'application/json'
      }
    },

    encodeParameters: function(xhr, parameters, elt) {
      xhr.overrideMimeType('text/json')

      const vals = api.getExpressionVars(elt)
      const object = {}
      parameters.forEach(function(value, key) {
        // FormData encodes values as strings, restore hx-vals/hx-vars with their initial types
        const typedValue = Object.hasOwn(vals, key) ? vals[key] : value
        const isArrayField = key.endsWith('[]')
        const cleanKey = isArrayField ? key.slice(0, -2) : key
        if (Object.hasOwn(object, cleanKey)) {
          if (!Array.isArray(object[cleanKey])) {
            object[cleanKey] = [object[cleanKey]]
          }
          object[cleanKey].push(typedValue)
        } else {
          object[cleanKey] = isArrayField ? [typedValue] : typedValue
        }
      })

      return (JSON.stringify(object))
    }
  })
})()
