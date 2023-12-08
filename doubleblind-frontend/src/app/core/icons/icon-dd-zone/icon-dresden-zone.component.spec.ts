import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IconDresdenZoneComponent } from './icon-dresden-zone.component';

describe('IconTudComponent', () => {
  let component: IconDresdenZoneComponent;
  let fixture: ComponentFixture<IconDresdenZoneComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [IconDresdenZoneComponent]
    });
    fixture = TestBed.createComponent(IconDresdenZoneComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
