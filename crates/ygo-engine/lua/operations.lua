-- Reports what an activated effect declares it will or may do to the
-- opponent's cards (Duel.SetOperationInfo, Duel.SetPossibleOperationInfo:
-- category, how many, from where), one log line per effect and declaration
-- in a duel, for operations.rs. The declaration itself is passed on
-- unchanged first.
local df_set_operation=Duel.SetOperationInfo
local df_set_possible=Duel.SetPossibleOperationInfo
local df_seen={}
-- Categories whose last parameter is a count of cards, not a location.
local df_counted={[CATEGORY_HANDES]=LOCATION_HAND,[CATEGORY_DECKDES]=LOCATION_DECK}
local function df_report(ct,cate,targets,count,p,param)
  local e,tp=Duel.GetChainInfo(ct,CHAININFO_TRIGGERING_EFFECT,CHAININFO_TRIGGERING_PLAYER)
  if not e or not tp then return end
  local c=e:GetHandler()
  if not c then return end
  local loc,n=0,count or 0
  if targets then
    -- The cards named: those of the opponent, wherever they are.
    local g=Group.CreateGroup()
    g:Merge(targets)
    local theirs=g:Filter(Card.IsControler,nil,1-tp)
    if #theirs==0 then return end
    local tc=theirs:GetFirst()
    while tc do
      loc=loc|tc:GetLocation()
      tc=theirs:GetNext()
    end
    n=#theirs
  else
    if p~=1-tp and p~=PLAYER_ALL and p~=PLAYER_EITHER then return end
    if df_counted[cate] then
      loc=df_counted[cate]
      if (param or 0)>n then n=param end
    else
      loc=param or 0
    end
  end
  if loc==0 or n<=0 then return end
  local p1=e:GetProperty()
  local key=string.format("%d %d %d %d %d %d",c:GetOriginalCode(),e:GetDescription(),cate,n,loc,e:GetCode())
  if df_seen[key] then return end
  df_seen[key]=true
  Debug.Message(string.format("dfo %d %d %d %d %d %d %d %d",
    c:GetOriginalCode(),e:GetDescription(),e:GetType(),e:GetCode(),cate,n,loc,p1&EFFECT_FLAG_CARD_TARGET))
end
Duel.SetOperationInfo=function(ct,cate,targets,count,p,param,...)
  df_set_operation(ct,cate,targets,count,p,param,...)
  pcall(df_report,ct,cate,targets,count,p,param)
end
-- Hyperinvoked Aeon declares so the 3 cards of the opponent's Extra Deck.
Duel.SetPossibleOperationInfo=function(ct,cate,targets,count,p,param,...)
  df_set_possible(ct,cate,targets,count,p,param,...)
  pcall(df_report,ct,cate,targets,count,p,param)
end
