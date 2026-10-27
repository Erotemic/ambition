"""Prepared identities keep forms, parts and independent derivations distinct."""
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from art_index.inventory import assemble, select


def test_identity_union_keeps_missing_bundles_and_explicit_relations(tmp_path):
    def unit(name):
        return {'id':'target:'+name,'kind':'actor','root':'sprites','generator':{'id':name,'family':'sprite_target'},
                'required':[name+'_spritesheet.ron'],'optional':[],'source':{'visual_family':'crew','species':'human'},
                'status':{'state':'missing','tiers':{}},'repair':'publish '+name}
    def actor(name, target, **relations):
        return {'id':name,'provider':'game','sheet_target':target,'catalog':{'display_name':name,'manifest':'sprites/'+target+'_spritesheet.ron','tags':['military'],**relations},'required_actions':['death']}
    registered = {'units':{'target:'+name:unit(name) for name in ['base','old','unused']},'complete':True,'warnings':[]}
    prepared = {'state':'available','characters':[actor('person','base'),actor('old_look','old',form_of='person'),
                actor('head','base',part_of='person'),actor('robot','base',derived_from='person'),actor('absent','unpublished')], 'sheets':{},'assets':[]}
    report=assemble(tmp_path,registered,prepared,local_files=[tmp_path/'orphan.png'],lineages={'subject':{'ids':['base','old'],'eras':[]}})
    assert len(report['identities'])==5
    assert report['units']['target:base']['identities']==['game:person','game:head','game:robot']
    assert report['identities']['game:robot']['relations']['derived_from']=='person'
    groups = {row['root']: set(row['members']) for row in report['character_groups']}
    assert groups['game:person'] == {'game:person', 'game:head', 'game:old_look'}
    assert groups['game:robot'] == {'game:robot'}
    assert groups['game:absent'] == {'game:absent'}
    assert report['units']['target:old']['game_use']=='available-look'
    assert report['units']['target:unused']['game_use']=='no-known-use'
    assert report['identities']['game:absent']['coverage'][0]['resolution']['state']=='missing-product'
    assert report['units']['target:base']['facts']['identity_kind']['state']=='unknown'
    assert report['local_products'][0]['registration']=='unregistered'
    assert {u['id'] for u in select(report,game_only=True,needs=True)}=={'target:base','target:old','requirement:game:absent'}
    registered['complete']=False
    assert assemble(tmp_path,registered,prepared,local_files=[tmp_path/'orphan.png'],lineages={})['local_products'][0]['registration']=='unknown'


def test_coverage_keeps_source_profile_and_runtime_fallback_separate(tmp_path):
    unit={'id':'target:body','kind':'actor','root':'sprites','generator':{'id':'body'},'required':['body_spritesheet.ron'],'optional':[],
          'source':{'available':['idle','death'],'selected':['idle']},'status':{'state':'current','tiers':{}},'repair':'publish'}
    prepared={'state':'available','characters':[{'id':'body','provider':'game','sheet_target':'body','catalog':{'manifest':'sprites/body_spritesheet.ron'},'required_actions':['death']}],
              'sheets':{str(tmp_path/'crates/ambition_platformer2d_actor_monolith/assets/sprites/body_spritesheet.ron'):[{'key':'body','product':'body','target':'toon','rows':[{'name':'idle','valid':True}],
                  'pages':[{'exists':True}],'actions':{'death':{'state':'fallback','row':'idle','steps':['Death','Hit','Idle']}}}]}}
    report=assemble(tmp_path,{'units':{'target:body':unit},'complete':True},prepared,lineages={})
    coverage=report['identities']['game:body']['coverage'][0]
    assert coverage['source_available'] and not coverage['profile_selected']
    assert coverage['resolution']['state']=='fallback' and coverage['resolution']['row']=='idle'
    assert select(report,missing='death')


def test_semantic_source_support_uses_the_sheet_owners_alias_projection():
    from art_index.inventory import supports
    semantics = {'hit':'Hit','hurt':'Hit','idle':'Idle'}
    assert supports({'available':['hurt']}, 'available', 'hit', semantics)
    assert not supports({'selected':['idle']}, 'selected', 'hit', semantics)
    assert supports({}, 'available', 'hit', semantics) is None


def test_referenced_prop_art_is_visible_before_a_producer_or_file_exists(tmp_path):
    prepared={'state':'available','characters':[],'sheets':{},'assets':[{'path':'sprites/chair.png','required_by':['game:prop/chair']}]}
    old={'root':'sprites','generator':{'id':'retired_chair'},'outputs':[{'path':'chair.png','sha256':'old'}],'basis':'rendered'}
    from art_index.paths import asset_root
    report=assemble(tmp_path,{'units':{},'complete':True,'receipts':[old]},prepared,
        local_files=[asset_root(tmp_path)/'sprites/chair.png'],lineages={'chair':{'ids':['retired_chair']}})
    unit=report['units']['requirement:asset:sprites/chair.png']
    assert unit['kind']=='prop' and unit['registration']=='unregistered'
    assert unit['status']['state']=='missing' and unit['lineages']==['chair']
    assert select(report,needs=True,game_only=True)==[unit]
    assert report['local_products'][0]['registration']=='unregistered'
    assert report['local_products'][0]['lineages']==['chair']


def test_one_publisher_can_serve_two_independent_lineages(tmp_path):
    unit = {'id': 'target:combined', 'kind': 'actor', 'root': 'sprites',
            'generator': {'id': 'combined'}, 'required': ['giant_spritesheet.ron', 'scholar_spritesheet.ron'],
            'optional': [], 'source': {}, 'status': {'state': 'current', 'tiers': {}}, 'repair': ''}
    prepared = {'state': 'available', 'characters': [
        {'id': name, 'provider': 'game', 'sheet_target': 'combined',
         'catalog': {'manifest': f'sprites/{name}_spritesheet.ron'}, 'required_actions': []}
        for name in ('giant', 'scholar')], 'sheets': {}}
    lineages = {name: {'ids': ['combined'], 'current_products': [f'sprites/{name}_spritesheet.ron'],
                       'split_from': ['origin']} for name in ('giant', 'scholar')}
    lineages['origin'] = {'ids': ['combined'], 'current_products': []}
    report = assemble(tmp_path, {'units': {'target:combined': unit}}, prepared, lineages=lineages)
    assert report['identities']['game:giant']['lineages'] == ['giant']
    assert report['identities']['game:scholar']['lineages'] == ['scholar']
    assert len(report['character_groups']) == 2
