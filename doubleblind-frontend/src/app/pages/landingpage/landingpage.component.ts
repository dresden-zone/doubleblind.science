import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import {ButtonComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";
import {IconGithubComponent} from "../../core/icons/icon-github/icon-github.component";
import {IconTudComponent} from "../../core/icons/icon-tud/icon-tud.component";
import {IconDresdenZoneComponent} from "../../core/icons/icon-dd-zone/icon-dresden-zone.component";
import {IconLasrComponent} from "../../core/icons/icon-lasr/icon-lasr.component";
import {IconAddComponent} from "../../core/icons/icon-add/icon-add.component";

@Component({
  selector: 'app-landingpage',
  standalone: true,
  imports: [CommonModule, ButtonComponent, CardComponent, IconGithubComponent, IconTudComponent, IconDresdenZoneComponent, IconLasrComponent, IconAddComponent],
  templateUrl: './landingpage.component.html',
  styleUrl: './landingpage.component.scss'
})
export class LandingpageComponent {
  protected call() {
    location.href='https://github.com/apps/doubleblind-science-testing';
  }
}
