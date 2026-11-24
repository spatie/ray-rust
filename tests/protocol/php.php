<?php

// Reference: spatie/ray dd0a28063c15e847f5d67a5855c97c273ddfe693.
require $argv[1] . '/vendor/autoload.php';

use Spatie\Ray\Client;
use Spatie\Ray\Ray;
use Spatie\Ray\Settings\Settings;

$ray = new Ray(new Settings([]), new Client((int) $argv[2], '127.0.0.1'));
$ray->project('Protocol')
    ->send('Hello world', 42, true, null, false, 0, '')
    ->green()->orange()->yellow()->red()->purple()->blue()->gray()->grey()
    ->small()->large()->label('Beatles')->send('updated')->hide()->remove()
    ->newScreen('My debug screen')->screenGreen()->screenOrange()->screenRed()
    ->screenPurple()->screenBlue()->screenGray()->clearScreen()->clearAll()
    ->separator()->notify('This is my notification')->confetti()->showApp()->hideApp()
    ->html('<b>Bold string</b>')->text("  <em>A & B's \"text\"</em>\n   preserved")
    ->sendCustom('my custom content', 'hey')
    // url() relies on an optional framework string helper; compare its HTML payload directly.
    ->html("<a href='https://spatie.be'>Spatie homepage</a>")
    ->table(['First' => 'First value', 'Second' => 2, 'Third' => true, 'Last' => null])
    ->table(['John', 'Paul', 'George', 'Ringo'], 'Beatles')
    ->toJson(['a' => 1, 'b' => ['c' => 3], 'url' => 'https://spatie.be', 'name' => 'Zoë 🎉'])
    ->image('https://placekitten.com/200/300')
    ->image('aGVsbG8=')->image('data:image/jpeg;base64,aGVsbG8=')
    ->expand(3)->expand('myKey', 'anotherKey.nestedKey')->expandAll()
    ->raw(true)->raw(null)->raw('raw')
    ->send(11111111111111111)
    ->xml('<one><two><three>3</three></two></one>')
    ->xml('<one/>')
    ->file('tests/fixtures/content.txt')
    ->file('tests/fixtures/missing.txt');
$ray->disable()->send('not sent')->enable()->if(false)->send('not sent either')
    ->if(true)->text('shown')->removeWhen(false)->showIf(false);
ray()->count('first');
ray()->count('first');
ray()->count('second');
for ($i = 1; $i <= 4; $i++) { ray()->limit(2)->send($i); }
for ($i = 1; $i <= 3; $i++) { ray()->once()->send($i); }
Ray::rateLimiter()->clear()->max(1);
ray('last allowed');
ray('rate limited');
ray('rate limited again');
