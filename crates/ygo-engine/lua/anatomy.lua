-- Reports every effect a card registers on itself while its script sets it
-- up, one log line each, for the anatomy of its effects (anatomy.rs). Loaded
-- only in reading duels, after the base scripts.
local df_register=Card.RegisterEffect
local function df_has(f)
  if f then return 1 end
  return 0
end
Card.RegisterEffect=function(c,e,...)
  local _,max,code,flag,hopt=e:GetCountLimit()
  local p1,p2=e:GetProperty()
  local s,o=e:GetTargetRange()
  local value=e:GetValue()
  if math.type(value)~="integer" then value=0 end
  Debug.Message(string.format("dfa %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d",
    c:GetOriginalCode(),e:GetType(),e:GetCode(),e:GetCategory(),p1,p2,e:GetRange(),s,o,
    max,code,flag,hopt,e:GetDescription(),value,
    df_has(e:GetCost()),df_has(e:GetCondition()),df_has(e:GetTarget()),df_has(e:GetOperation())))
  return df_register(c,e,...)
end
