--!strict

-- https://luau-lang.org/getting-started

print("hello world!")

function is_positive(x: number) : boolean
    return x > 0
end

local result: boolean

result = is_positive(1)

print(result)

local client = reqwest()

print("before")

local asdf = client:get("https://httpbin.org/ip")

print(asdf)

print("after")
