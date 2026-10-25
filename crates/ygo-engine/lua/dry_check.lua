-- Dry check of your cards' effects at the start of the opponent's turn, with
-- no opponent playing (dry.rs, docs/plans/linee-interazione.md §3.2): each
-- effect that could be used then has its condition, cost and target asked
-- "can it be done?" (chk == 0), which changes nothing. Questions about your
-- side are answered by the duel; questions about the opponent's side or the
-- event that would trigger the effect are answered yes, and reported as
-- assumptions. One log line per effect; everything wrapped is put back.
-- df_tp, the player checked, is set by the line before this file.
local df_s={}
local df_saved={}
local function df_wrap(lib,name,make)
  local f=lib[name]
  if not f then return end
  df_saved[#df_saved+1]={lib,name,f}
  lib[name]=make(f)
end

-- A stand-in for the event's cards, effect and group: it answers yes to what
-- is asked of it, and tells that the effect looked at the event.
local df_proxy
local df_no={IsDisabled=true,IsStatus=true,IsImmuneToEffect=true,IsHasEffect=true}
local df_card={GetHandler=true,GetFirst=true,GetOwner=true,GetReasonCard=true,GetAttacker=true,GetBattleTarget=true}
local function df_method(name)
  return function()
    df_s.event=true
    if df_no[name] then return false end
    if name:find("^Is") or name:find("^Check") or name:find("^Has") then return true end
    if name=="GetCount" then return 1 end
    if name=="GetNext" then return nil end
    if df_card[name] then return df_proxy end
    if name:find("Player$") or name=="GetControler" or name=="GetHandlerPlayer" then return 1-df_tp end
    if name:find("^Get") then return 0 end
    return df_proxy
  end
end
df_proxy=setmetatable({},{
  __index=function(_,k) df_s.event=true return df_method(k) end,
  __len=function() df_s.event=true return 1 end,
})
local function df_clean(ex)
  if ex==df_proxy then return nil end
  return ex
end
-- Your locations and the opponent's in a query asked for player p.
local function df_sides(p,s,o)
  if p==df_tp then return s or 0,o or 0 end
  return o or 0,s or 0
end
local function df_ids(g)
  local out={}
  for tc in aux.Next(g) do out[#out+1]=tc:GetFieldID() end
  return out
end
local function df_demand(count,g)
  if df_s.in_cost and type(count)=="number" then
    df_s.demands[#df_s.demands+1]=string.format("%d:%s",count,table.concat(df_ids(g),","))
  end
end
local df_group=Duel.GetMatchingGroup

df_wrap(Duel,"IsExistingMatchingCard",function(f) return function(filter,p,s,o,count,ex,...)
  local own,opp=df_sides(p,s,o)
  if opp~=0 then df_s.opp=true return true end
  ex=df_clean(ex)
  -- A query that depends on the event (a card "of the same type as the
  -- activated one", read from it before or in the filter) cannot be
  -- answered: any of your cards there may do.
  local before=df_s.event
  df_s.event=false
  local ok=f(filter,df_tp,own,0,count,ex,...)
  local g=df_group(filter,df_tp,own,0,ex,...)
  local open=before or df_s.event
  df_s.event=open
  if open then
    g=df_group(aux.TRUE,df_tp,own,0,ex)
    ok=ok or #g>=(count or 1)
  end
  df_demand(count,g)
  if not ok then df_s.own_false=true end
  return ok
end end)
-- Outside a chain the engine has no effect being checked, and asking it
-- whether a card can be targeted with none crashes: the targets are counted
-- here, against the effect checked.
local df_targetable=Card.IsCanBeEffectTarget
df_wrap(Card,"IsCanBeEffectTarget",function(f) return function(c,e,...)
  return f(c,e or df_s.effect,...)
end end)
df_wrap(Duel,"IsExistingTarget",function() return function(filter,p,s,o,count,ex,...)
  local own,opp=df_sides(p,s,o)
  if opp~=0 then df_s.opp=true return true end
  filter=filter or aux.TRUE
  ex=df_clean(ex)
  local before=df_s.event
  df_s.event=false
  local n=df_group(function(tc,...) return df_targetable(tc,df_s.effect) and filter(tc,...) end,df_tp,own,0,ex,...):GetCount()
  if before or df_s.event then
    n=df_group(function(tc) return df_targetable(tc,df_s.effect) end,df_tp,own,0,ex):GetCount()
  end
  df_s.event=before or df_s.event
  local ok=n>=(count or 1)
  if not ok then df_s.own_false=true end
  return ok
end end)
df_wrap(Duel,"GetMatchingGroup",function(f) return function(filter,p,s,o,ex,...)
  local own,opp=df_sides(p,s,o)
  if opp~=0 then df_s.opp=true end
  return f(filter,df_tp,own,0,df_clean(ex),...)
end end)
df_wrap(Duel,"GetMatchingGroupCount",function(f) return function(filter,p,s,o,ex,...)
  local own,opp=df_sides(p,s,o)
  local n=f(filter,df_tp,own,0,df_clean(ex),...)
  if opp~=0 then df_s.opp=true n=n+99 end
  return n
end end)
df_wrap(Duel,"GetFieldGroup",function(f) return function(p,s,o)
  local own,opp=df_sides(p,s,o)
  if opp~=0 then df_s.opp=true end
  return f(df_tp,own,0)
end end)
df_wrap(Duel,"GetFieldGroupCount",function(f) return function(p,s,o)
  local own,opp=df_sides(p,s,o)
  local n=f(df_tp,own,0)
  if opp~=0 then df_s.opp=true n=n+99 end
  return n
end end)
df_wrap(Duel,"IsPlayerAffectedByEffect",function(f) return function(p,...)
  if p~=df_tp then df_s.opp=true return nil end
  return f(p,...)
end end)
-- The chain and the battle are the event's.
for _,name in ipairs({"IsChainNegatable","IsChainDisablable"}) do
  df_wrap(Duel,name,function() return function() df_s.event=true return true end end)
end
for _,name in ipairs({"GetOperationInfo","GetPossibleOperationInfo"}) do
  df_wrap(Duel,name,function() return function() df_s.event=true return false end end)
end
df_wrap(Duel,"GetCurrentChain",function() return function() df_s.event=true return 1 end end)
for _,name in ipairs({"GetAttacker","GetAttackTarget","GetChainEvent"}) do
  df_wrap(Duel,name,function() return function() df_s.event=true return df_proxy end end)
end
df_wrap(Duel,"GetChainInfo",function() return function(_,...)
  df_s.event=true
  local out={}
  for i,info in ipairs({...}) do
    if info==CHAININFO_TRIGGERING_PLAYER or info==CHAININFO_TRIGGERING_CONTROLER then out[i]=1-df_tp
    elseif info==CHAININFO_TRIGGERING_LOCATION then out[i]=LOCATION_ONFIELD
    else out[i]=df_proxy end
  end
  return table.unpack(out,1,select("#",...))
end end)
-- The phase of the opponent's turn is the one the effect needs.
df_wrap(Duel,"GetCurrentPhase",function() return function() df_s.phase_read=true return df_s.phase end end)
-- Costs: what they would use.
for _,name in ipairs({"IsDiscardable","IsAbleToGraveAsCost","IsAbleToRemoveAsCost","IsAbleToDeckAsCost",
    "IsAbleToHandAsCost","IsAbleToExtraAsCost","IsReleasable"}) do
  df_wrap(Card,name,function(f) return function(c,...)
    local ok=f(c,...)
    if df_s.in_cost and c==df_s.handler then df_s.self_cost=true end
    return ok
  end end)
end
df_wrap(Card,"CheckRemoveOverlayCard",function(f) return function(c,p,count,...)
  local ok=f(c,p,count,...)
  df_demand(count,c:GetOverlayGroup())
  if not ok then df_s.own_false=true end
  return ok
end end)
df_wrap(Duel,"CheckRemoveOverlayCard",function(f) return function(p,s,o,count,...)
  local ok=f(p,s,o,count,...)
  df_demand(count,Duel.GetOverlayGroup(df_tp,1,0))
  if not ok then df_s.own_false=true end
  return ok
end end)
df_wrap(Duel,"CheckReleaseGroupCost",function(f) return function(p,filter,count,...)
  local ok=f(p,filter,count,...)
  df_demand(count,df_group(Card.IsReleasable,df_tp,LOCATION_MZONE,0,nil))
  if not ok then df_s.own_false=true end
  return ok
end end)

local DF_ACTIVATED=EFFECT_TYPE_ACTIVATE|EFFECT_TYPE_QUICK_O|EFFECT_TYPE_QUICK_F|EFFECT_TYPE_TRIGGER_O|EFFECT_TYPE_TRIGGER_F
local DF_PHASES={PHASE_MAIN1,PHASE_BATTLE_STEP,PHASE_END}

-- Calls f under the stand-ins; true when it says yes or cannot tell.
local function df_ask(stage,f,...)
  if not f then return true end
  df_s.event,df_s.opp,df_s.own_false=false,false,false
  local ok,res=pcall(f,...)
  if not ok then df_s.flags=df_s.flags|8 return true end
  if df_s.opp then df_s.flags=df_s.flags|1 end
  if df_s.event then df_s.flags=df_s.flags|2 end
  if res then return true end
  -- A no that comes only from the opponent's side or the event is not
  -- known: it is assumed yes. A no from your side is the answer.
  if (df_s.opp or df_s.event) and not df_s.own_false then return true end
  df_s.reason=stage
  return false
end

local function df_check(c,e)
  local type_=e:GetType()
  local activated=type_&DF_ACTIVATED~=0
  local range=e:GetRange()
  if activated then
    if type_&EFFECT_TYPE_ACTIVATE~=0 then
      if not (c:IsLocation(LOCATION_SZONE) and c:IsFacedown()) then return 1 end
      if not (c:IsType(TYPE_TRAP) or c:IsType(TYPE_QUICKPLAY)) then return 1 end
    elseif range==0 or not c:IsLocation(range) then
      return 1
    end
  elseif type_&EFFECT_TYPE_FIELD==0 or range==0 or not c:IsLocation(range) then
    return 1
  end
  if c:IsOnField() and type_&EFFECT_TYPE_ACTIVATE==0 then
    if c:IsFacedown() then return 2 end
    if c:IsDisabled() then return 3 end
  end
  if activated and not e:CheckCountLimit(df_tp) then return 4 end
  if not activated then
    for _,phase in ipairs(DF_PHASES) do
      df_s.phase,df_s.phase_read=phase,false
      df_s.reason=0
      if df_ask(5,e:GetCondition(),e) then return 0 end
      if not df_s.phase_read then break end
    end
    return df_s.reason
  end
  local eg,ep,ev,re,r,rp=nil,0,0,nil,0,0
  if e:GetCode()~=EVENT_FREE_CHAIN then
    eg,ep,ev,re,r,rp=df_proxy,1-df_tp,1,df_proxy,REASON_EFFECT,1-df_tp
  end
  local passed=false
  for _,phase in ipairs(DF_PHASES) do
    df_s.phase,df_s.phase_read=phase,false
    if df_ask(5,e:GetCondition(),e,df_tp,eg,ep,ev,re,r,rp) then passed=true break end
    if not df_s.phase_read then break end
    df_s.flags=df_s.flags|4
  end
  -- Every step is asked even after a no: what the later ones look at
  -- (the opponent's cards) tells what the effect does.
  df_s.in_cost=true
  local paid=df_ask(6,e:GetCost(),e,df_tp,eg,ep,ev,re,r,rp,0)
  df_s.in_cost=false
  local targets=df_ask(7,e:GetTarget(),e,df_tp,eg,ep,ev,re,r,rp,0,nil)
  if not passed then return 5 end
  if not paid then return 6 end
  if not targets then return 7 end
  return 0
end

local function df_has(f)
  if f then return 1 end
  return 0
end
-- The engine keeps few log lines: the effects go many to a message.
local df_lines={}
local function df_report(c,e)
  df_s.flags,df_s.reason,df_s.demands,df_s.handler,df_s.self_cost=0,0,{},c,false
  df_s.effect=e
  df_s.phase=PHASE_MAIN1
  local ok,reason=pcall(df_check,c,e)
  if not ok then reason=0 df_s.flags=df_s.flags|8 end
  if reason==1 then return end
  if df_s.self_cost then
    df_s.flags=df_s.flags|16
    df_s.demands[#df_s.demands+1]="1:"..c:GetFieldID()
  end
  local demands="-"
  if #df_s.demands>0 then demands=table.concat(df_s.demands,";") end
  local _,max,code,flag,hopt=e:GetCountLimit()
  local p1,p2=e:GetProperty()
  local s,o=e:GetTargetRange()
  local value=e:GetValue()
  if math.type(value)~="integer" then value=0 end
  df_lines[#df_lines+1]=string.format("dfd %d %d %d %d %d %s | dfa %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d",
    c:GetFieldID(),c:GetLocation(),c:GetSequence(),reason,df_s.flags,demands,
    c:GetOriginalCode(),e:GetType(),e:GetCode(),e:GetCategory(),p1,p2,e:GetRange(),s,o,
    max or 0,code or 0,flag or 0,hopt or 0,e:GetDescription(),value,
    df_has(e:GetCost()),df_has(e:GetCondition()),df_has(e:GetTarget()),df_has(e:GetOperation()))
end

local ok,err=pcall(function()
  local g=df_group(aux.TRUE,df_tp,LOCATION_HAND|LOCATION_ONFIELD|LOCATION_GRAVE|LOCATION_REMOVED,0,nil)
  for c in aux.Next(g) do
    for _,e in ipairs({c:GetOwnEffects()}) do df_report(c,e) end
  end
end)
for i=#df_saved,1,-1 do
  local w=df_saved[i]
  w[1][w[2]]=w[3]
end
for i=1,#df_lines,25 do
  Debug.Message(table.concat(df_lines,"\n",i,math.min(i+24,#df_lines)))
end
if not ok then Debug.Message("dfd error "..tostring(err)) end
Debug.Message("dfd end")
