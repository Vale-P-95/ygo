-- The reading duels of the synergies (reading.rs). Loaded only there,
-- after the base scripts: nothing of a real duel changes.
-- 1. The materials each Extra Deck monster asks for, as its script hands
--    them to the summon procedures; then which cards each material accepts,
--    by calling the script's own filter on every monster of the duel.
-- 2. The limits an effect puts on its player ("you cannot Special Summon,
--    except...", "you cannot Normal Summon"), with the cards they still allow, by calling the limit's
--    own target on every card of the duel.
-- Cards are told by bitmaps over every card code of the duel, sorted and
-- without repeats (the reading deck holds the whole pool).
if df_reading then return end
df_reading=true
df_mat={}
local function df_record(c,kind,slots,check)
	if c:IsStatus(STATUS_COPYING_EFFECT) then return end
	local code=c:GetOriginalCode()
	df_mat[code]=df_mat[code] or {card=c,procs={}}
	table.insert(df_mat[code].procs,{kind=kind,slots=slots,check=check})
end
local function df_count(n)
	if type(n)~="number" then return 99 end
	return n
end
-- A Fusion material as the procedure takes it: a filter, a card code or
-- a table of both.
local function df_fusion_slot(v)
	local st=SUMMON_TYPE_FUSION|MATERIAL_FUSION
	if type(v)=="function" then
		return function(mc,sc) return v(mc,sc,st,0) end
	elseif type(v)=="table" then
		local parts={}
		for _,x in ipairs(v) do table.insert(parts,df_fusion_slot(x)) end
		return function(mc,sc)
			for _,f in ipairs(parts) do if f(mc,sc) then return true end end
			return false
		end
	end
	return function(mc,sc) return mc:IsSummonCode(sc,st,0,v) end
end
local df_mix=Fusion.AddProcMix
Fusion.AddProcMix=function(c,sub,insf,...)
	local slots={}
	for _,v in ipairs({...}) do table.insert(slots,{fn=df_fusion_slot(v),min=1,max=1}) end
	df_record(c,"fusion",slots,false)
	return df_mix(c,sub,insf,...)
end
local df_mixrep=Fusion.AddProcMixRep
Fusion.AddProcMixRep=function(c,sub,insf,fun1,minc,maxc,...)
	local slots={{fn=df_fusion_slot(fun1),min=minc,max=df_count(maxc)}}
	for _,v in ipairs({...}) do table.insert(slots,{fn=df_fusion_slot(v),min=1,max=1}) end
	df_record(c,"fusion",slots,false)
	return df_mixrep(c,sub,insf,fun1,minc,maxc,...)
end
local df_synchro=Synchro.AddProcedure
Synchro.AddProcedure=function(c,...)
	local f1,min1,max1,f2,min2,max2,sub1,sub2,req1,req2,reqm=...
	local st=SUMMON_TYPE_SYNCHRO|MATERIAL_SYNCHRO
	if type(min1)=="number" then
		df_record(c,"synchro",{
			{fn=function(mc,sc) return mc:IsType(TYPE_TUNER,sc,st,0) and (not f1 or f1(mc,sc,st,0)) end,min=min1,max=df_count(max1)},
			{fn=function(mc,sc) return not f2 or f2(mc,sc,st,0) end,min=min2 or 1,max=df_count(max2)},
		},(sub1 or sub2 or req1 or req2 or reqm) and true or false)
	end
	return df_synchro(c,...)
end
local df_majestic=Synchro.AddMajesticProcedure
Synchro.AddMajesticProcedure=function(c,f1,cbt1,f2,cbt2,f3,cbt3,...)
	local st=SUMMON_TYPE_SYNCHRO|MATERIAL_SYNCHRO
	local slots={}
	for _,f in ipairs({f1,f2,f3}) do
		table.insert(slots,{fn=function(mc,sc) return f(mc,sc,st,0) end,min=1,max=1})
	end
	df_record(c,"synchro",slots,select('#',...)>0)
	return df_majestic(c,f1,cbt1,f2,cbt2,f3,cbt3,...)
end
local df_dark=Synchro.AddDarkSynchroProcedure
Synchro.AddDarkSynchroProcedure=function(c,f1,f2,plv,nlv,...)
	local st=SUMMON_TYPE_SYNCHRO|MATERIAL_SYNCHRO
	df_record(c,"synchro",{
		{fn=function(mc,sc) return mc:IsType(TYPE_TUNER,sc,st,0) and (not f1 or f1(mc,sc,st,0)) end,min=1,max=1},
		{fn=function(mc,sc) return not f2 or f2(mc,sc,st,0) end,min=1,max=1},
	},true)
	return df_dark(c,f1,f2,plv,nlv,...)
end
local df_xyz=Xyz.AddProcedure
Xyz.AddProcedure=function(c,f,lv,ct,alterf,desc,maxct,op,mustbemat,exchk)
	local st=SUMMON_TYPE_XYZ|MATERIAL_XYZ
	local slots={{fn=function(mc,sc)
		return (not f or f(mc,sc,st,0)) and (not lv or mc:IsXyzLevel(sc,lv))
	end,min=ct,max=df_count(maxct or ct)}}
	df_record(c,"xyz",slots,(mustbemat or exchk) and true or false)
	if alterf then
		df_record(c,"xyz on a monster",{{fn=function(mc,sc) return alterf(mc,0,sc) end,min=1,max=1}},op and true or false)
	end
	return df_xyz(c,f,lv,ct,alterf,desc,maxct,op,mustbemat,exchk)
end
local df_link=Link.AddProcedure
Link.AddProcedure=function(c,f,min,max,specialchk,desc,spcon)
	local st=SUMMON_TYPE_LINK|MATERIAL_LINK
	df_record(c,"link",{{fn=function(mc,sc) return not f or f(mc,sc,st,0) end,min=min,max=df_count(max or c:GetLink())}},specialchk and true or false)
	return df_link(c,f,min,max,specialchk,desc,spcon)
end

-- Every card code of the duel once, sorted, with one card of each: the
-- order of the bitmaps. Kept until df_reset (one reading of materials).
local df_cards=nil
function df_reset() df_cards=nil end
local function df_pool()
	if df_cards then return df_cards end
	local seen={}
	df_cards={}
	for tc in Duel.GetFieldGroup(0,LOCATION_DECK|LOCATION_EXTRA|LOCATION_HAND|LOCATION_GRAVE|LOCATION_REMOVED|LOCATION_ONFIELD,0):Iter() do
		local code=tc:GetOriginalCode()
		if not seen[code] then
			seen[code]=true
			table.insert(df_cards,tc)
		end
	end
	table.sort(df_cards,function(a,b) return a:GetOriginalCode()<b:GetOriginalCode() end)
	return df_cards
end
local df_hex="0123456789abcdef"
-- The bitmap of the cards for which `test` is true, and how many calls failed.
local function df_bits(cards,test)
	local out={}
	local errors=0
	local nibble=0
	for i,mc in ipairs(cards) do
		local fine,res=pcall(test,mc)
		if not fine then errors=errors+1 elseif res then nibble=nibble|(1<<((i-1)%4)) end
		if i%4==0 then
			table.insert(out,df_hex:sub(nibble+1,nibble+1))
			nibble=0
		end
	end
	if #cards%4~=0 then table.insert(out,df_hex:sub(nibble+1,nibble+1)) end
	return table.concat(out),errors
end
-- One line for each material of `code`: its procedure, the slot, how many,
-- whether the procedure checks more than each material alone, how many
-- filter calls failed, and which cards it accepts.
function df_mat_report(code)
	local entry=df_mat[code]
	if not entry then
		Debug.Message(string.format("dfm %d none",code))
		return
	end
	local cards=df_pool()
	for p,proc in ipairs(entry.procs) do
		for s,slot in ipairs(proc.slots) do
			local bits,errors=df_bits(cards,function(mc) return mc:IsMonster() and slot.fn(mc,entry.card) end)
			local check=0
			if proc.check then check=1 end
			Debug.Message(string.format("dfm %d %d %d %s %d %d %d %d %d %s",code,p,s,proc.kind:gsub(" ","_"),slot.min or 1,slot.max or 1,check,errors,#cards,bits))
		end
	end
end

-- The limits on the player: a line with the card that set it, the effect,
-- and the cards it forbids (a limit without a target forbids them all).
local df_limits={[EFFECT_CANNOT_SUMMON]=true,[EFFECT_CANNOT_SPECIAL_SUMMON]=true}
local df_register=Duel.RegisterEffect
Duel.RegisterEffect=function(e,p,...)
	local res=df_register(e,p,...)
	if p==0 and df_limits[e:GetCode()] and e:IsHasType(EFFECT_TYPE_FIELD) then
		local owner=e:GetOwner()
		local code=0
		if owner then code=owner:GetOriginalCode() end
		local target=e:GetTarget()
		df_cards=nil
		local cards=df_pool()
		local bits,errors
		if target then
			bits,errors=df_bits(cards,function(tc) return target(e,tc,0,0,POS_FACEUP,0,nil) end)
		else
			bits,errors=df_bits(cards,function() return true end)
		end
		df_cards=nil
		Debug.Message(string.format("dfl %d %d %d %d %s",code,e:GetCode(),errors,#cards,bits))
	end
	return res
end

-- 3. Helpers that make an event happen to the card read, so that its
--    triggers answer as in a duel: effects of a card of the hand that does
--    nothing, each moving the card read once (sends it to the GY, Special
--    Summons it, banishes it...). Their descriptions are DF_HELPER plus the
--    event; the reading activates them after the card's own actions.
DF_HELPER=0x7df00000
local function df_find(code)
	return Duel.GetFirstMatchingCard(function(c) return c:GetOriginalCode()==code end,0,LOCATION_HAND|LOCATION_ONFIELD|LOCATION_GRAVE|LOCATION_REMOVED,0,nil)
end
local df_moves={
	[EVENT_TO_GRAVE]={LOCATION_HAND|LOCATION_ONFIELD,function(c)
		local r=REASON_EFFECT
		if c:IsLocation(LOCATION_HAND) then r=r|REASON_DISCARD end
		Duel.SendtoGrave(c,r)
	end},
	[EVENT_DISCARD]={LOCATION_HAND,function(c) Duel.SendtoGrave(c,REASON_EFFECT|REASON_DISCARD) end},
	[EVENT_SPSUMMON_SUCCESS]={LOCATION_HAND|LOCATION_GRAVE|LOCATION_REMOVED,function(c) Duel.SpecialSummon(c,0,0,0,true,false,POS_FACEUP_ATTACK) end},
	[EVENT_SUMMON_SUCCESS]={LOCATION_HAND,function(c) Duel.Summon(0,c,true,nil) end},
	-- Destroyed, banished or bounced by the opponent: what a card that
	-- floats asks for ("destroyed by your opponent's card effect").
	[EVENT_REMOVE]={LOCATION_HAND|LOCATION_ONFIELD|LOCATION_GRAVE,function(c) Duel.Remove(c,POS_FACEUP,REASON_EFFECT,PLAYER_NONE,1) end},
	[EVENT_DESTROYED]={LOCATION_HAND|LOCATION_ONFIELD,function(c) Duel.Destroy(c,REASON_EFFECT,LOCATION_GRAVE,1) end},
	[EVENT_LEAVE_FIELD]={LOCATION_ONFIELD,function(c) Duel.SendtoHand(c,nil,REASON_EFFECT,1) end},
	[EVENT_TO_HAND]={LOCATION_GRAVE|LOCATION_REMOVED,function(c) Duel.SendtoHand(c,nil,REASON_EFFECT) end},
	[EVENT_TO_DECK]={LOCATION_HAND|LOCATION_ONFIELD|LOCATION_GRAVE|LOCATION_REMOVED,function(c) Duel.SendtoDeck(c,nil,SEQ_DECKSHUFFLE,REASON_EFFECT) end},
	[EVENT_RELEASE]={LOCATION_HAND|LOCATION_MZONE,function(c) Duel.Release(c,REASON_EFFECT) end},
	[EVENT_FLIP]={LOCATION_MZONE,function(c) Duel.ChangePosition(c,POS_FACEUP_DEFENSE) end},
}
function df_helpers(probe,holder,...)
	local h=Duel.GetFirstMatchingCard(function(c) return c:GetOriginalCode()==holder end,0,LOCATION_HAND,0,nil)
	if not h then return end
	for _,ev in ipairs({...}) do
		local move=df_moves[ev]
		if move then
			local e=Effect.CreateEffect(h)
			e:SetDescription(DF_HELPER+ev)
			e:SetType(EFFECT_TYPE_IGNITION)
			e:SetRange(LOCATION_HAND)
			e:SetTarget(function(e,tp,eg,ep,ev2,re,r,rp,chk)
				local c=df_find(probe)
				if chk==0 then
					if not c or not c:IsLocation(move[1]) then return false end
					if ev==EVENT_SPSUMMON_SUCCESS then return Duel.GetLocationCount(0,LOCATION_MZONE)>0 end
					if ev==EVENT_SUMMON_SUCCESS then return c:IsSummonable(true,nil) end
					if ev==EVENT_FLIP then return c:IsFacedown() end
					return true
				end
			end)
			e:SetOperation(function()
				local c=df_find(probe)
				if c and c:IsLocation(move[1]) then move[2](c) end
			end)
			h:RegisterEffect(e)
		end
	end
end
