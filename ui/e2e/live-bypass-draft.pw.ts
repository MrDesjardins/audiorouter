import {test, expect} from '@playwright/test';
import {demoSession} from '../src/fixtures';

for (const theme of ['dark', 'light', 'high-contrast']) {
  test(`live bypass preserves unsaved edits in ${theme}`, async ({page}, testInfo) => {
    const session = {...demoSession, edges: [
      {id:'mic-voice',sourceNode:'mic',sourcePort:'out',destinationNode:'voice',destinationPort:'in',matrix:[1],enabled:true},
      {id:'voice-output',sourceNode:'voice',sourcePort:'out',destinationNode:'headphones',destinationPort:'in',matrix:[1,1],enabled:true},
    ]};
    await page.addInitScript(({session,theme}) => {
      Object.assign(window,{__routeFixtureSession:session,__routeFixtureRunning:true});
      localStorage.setItem('audiorouter.ui.theme',theme);
    },{session,theme});
    await page.goto('/route-harness.html');
    await page.getByRole('combobox', {name:'Color theme'}).selectOption(theme);
    await expect(page.locator('.audio-run-state')).toContainText('Audio running');
    await page.getByTestId('rf__node-voice').locator('.flow-node-title').click();
    await page.getByLabel('Node name', {exact:true}).fill('Unsaved voice name');
    await page.getByLabel('Bypass', {exact:true}).check();
    await expect(page.locator('.global-action-message')).toContainText('applied to the playing audio');
    await page.waitForTimeout(1200);
    await expect(page.getByLabel('Node name', {exact:true})).toHaveValue('Unsaved voice name');
    await expect(page.getByLabel('Bypass', {exact:true})).toBeChecked();
    await expect(page.locator('.audio-run-state')).toContainText('Audio running');
    await page.screenshot({path:testInfo.outputPath(`2026-09-29-live-bypass-${theme}.png`)});
    await page.getByLabel('Bypass', {exact:true}).uncheck();
    await expect(page.locator('.global-action-message')).toContainText('applied to the playing audio');
    await expect(page.getByLabel('Node name', {exact:true})).toHaveValue('Unsaved voice name');
  });
}
