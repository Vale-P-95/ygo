-- Compares the replacements (saved in DF_FAST_*) with the upstream functions,
-- loaded after them, on groups of the Deck's monsters. Errors on the first
-- different answer.
local fast_loop,fast_check=DF_FAST_LOOP,DF_FAST_CHECK
local up_loop,up_check=Auxiliary.SelectUnselectLoop,Ritual.Check
assert(fast_loop~=up_loop and fast_check~=up_check,"the upstream functions are loaded")
local cards={}
for c in Duel.GetFieldGroup(0,LOCATION_DECK,0):Iter() do
	if c:IsType(TYPE_MONSTER) and #cards<8 then cards[#cards+1]=c end
end
assert(#cards==8,"8 monsters in the Deck")
local mg=Group.CreateGroup()
for _,c in ipairs(cards) do mg:AddCard(c) end
local function b(v) return v and true or false end

local rescons={
	false,
	function(sg) local s=sg:GetSum(Card.GetLevel) return s==8,s>8 end,
	function(sg) local n=sg:GetClassCount(Card.GetCode) return n==#sg and #sg>=2,n<#sg end,
	function(sg) return sg:FilterCount(Card.IsAttribute,nil,ATTRIBUTE_DARK)==2 end,
	function(sg,e,tp,mg,c) return c:IsLevelBelow(4) and #sg==3 end,
	function(sg) return #sg==1,#sg>1 end,
}
-- sums of Levels that some groups reach and others miss
for _,k in ipairs({3,5,7,11,13,17}) do
	rescons[#rescons+1]=function(sg) local s=sg:GetSum(Card.GetLevel) return s==k,s>k end
end
local bounds={{1,1},{1,2},{2,3},{1,4},{3,3},{2,8}}
for i,rescon in ipairs(rescons) do
	rescon=rescon or nil
	for _,mm in ipairs(bounds) do
		local minc,maxc=mm[1],mm[2]
		-- the open rescons try every order upstream: keep them small
		if i==2 or i==3 or i>6 or maxc<=4 then
			for _,first in ipairs({false,cards[1]}) do
				local sg=Group.CreateGroup()
				if first then sg:AddCard(first) end
				for _,c in ipairs(cards) do
					if not sg:IsContains(c) then
						local a=b(up_loop(c,sg,mg,nil,0,minc,maxc,rescon))
						local f=b(fast_loop(c,sg,mg,nil,0,minc,maxc,rescon))
						if a~=f then error(("SelectUnselectLoop differs: rescon %d, %d-%d, upstream %s"):format(i,minc,maxc,tostring(a))) end
					end
				end
			end
		end
	end
end

-- Ritual.Check: every group of up to 4 of the cards, each card as the last.
local forced={
	false,
	function(e,tp,g,sc) return {g:GetClassCount(Card.GetCode)==#g} end,
	function(e,tp,g,sc) local n=g:FilterCount(Card.IsLevelAbove,nil,4) return {n<=1,n>1} end,
}
local groups={}
local function walk(from,picked)
	if #picked>0 then groups[#groups+1]={table.unpack(picked)} end
	if #picked==4 then return end
	for j=from,#cards do
		picked[#picked+1]=cards[j]
		walk(j+1,picked)
		picked[#picked]=nil
	end
end
walk(1,{})
local sc=cards[1]
for _,_type in ipairs({RITPROC_EQUAL,RITPROC_GREATER}) do
	for _,fs in ipairs(forced) do
		for _,lv in ipairs({1,4,8,12}) do
			local up=up_check(sc,lv,fs or nil,_type)
			local fast=fast_check(sc,lv,fs or nil,_type)
			for _,picked in ipairs(groups) do
				local sg=Group.CreateGroup()
				for _,c in ipairs(picked) do sg:AddCard(c) end
				local last=picked[#picked]
				local r1,s1=up(sg,nil,0,mg,last)
				local r2,s2=fast(sg,nil,0,mg,last)
				if b(r1)~=b(r2) or b(s1)~=b(s2) then
					error(("Ritual.Check differs: type %d, level %d, %d cards"):format(_type,lv,#picked))
				end
			end
		end
	end
end
